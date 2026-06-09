use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{InternalEvent, MessageSource};
use crate::protocol::adapter::milky_to_internal_segments;
use crate::protocol::types::{ApiRequest, BotRuntimeContext};
use crate::services::ServiceHub;

/// Protocol-agnostic backend abstraction.
///
/// Implementations (e.g. `VirtualBackend`, future `OneBotBackend`) provide
/// event subscription and API dispatch.  `ProtocolServer` holds
/// `Arc<dyn ProtocolBackend>` so the active protocol can be switched at
/// runtime with low coupling.
#[async_trait]
pub trait ProtocolBackend: Send + Sync {
    /// Subscribe to the event bus for the bound user.
    fn subscribe_events(
        &self,
        bot: &BotRuntimeContext,
    ) -> AppResult<broadcast::Receiver<InternalEvent>>;

    /// Dispatch an API call and return raw JSON data.
    async fn call_api(
        &self,
        bot: &BotRuntimeContext,
        api: ApiRequest,
    ) -> AppResult<serde_json::Value>;
}

/// VirtualBackend implements `ProtocolBackend` by delegating to the
/// existing `ServiceHub` and `CoreContainer`.
#[derive(Clone)]
pub struct VirtualBackend {
    service_hub: ServiceHub,
    core: Arc<CoreContainer>,
}

impl VirtualBackend {
    pub fn new(service_hub: ServiceHub, core: Arc<CoreContainer>) -> Self {
        Self { service_hub, core }
    }
}

#[async_trait]
impl ProtocolBackend for VirtualBackend {
    fn subscribe_events(
        &self,
        bot: &BotRuntimeContext,
    ) -> AppResult<broadcast::Receiver<InternalEvent>> {
        let ctx = self
            .core
            .user_context(&bot.bound_user_id)
            .ok_or_else(|| AppError::not_found("bound user not registered"))?;
        Ok(ctx.event_tx.subscribe())
    }

    async fn call_api(
        &self,
        bot: &BotRuntimeContext,
        api: ApiRequest,
    ) -> AppResult<serde_json::Value> {
        match api.api_name.as_str() {
            "get_login_info" => {
                let user = self
                    .service_hub
                    .user
                    .get_user_by_id(&bot.bound_user_id)
                    .await?
                    .ok_or_else(|| AppError::not_found("bound user not found"))?;
                Ok(serde_json::json!({
                    "user_id": user.user_id.parse::<i64>().unwrap_or(0),
                    "nickname": user.nickname,
                }))
            }
            "send_private_message" => {
                let user_id = api
                    .params
                    .get("user_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing user_id"))?
                    .to_string();
                let message = api
                    .params
                    .get("message")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| AppError::validation("missing message"))?;
                let milky_segments: Vec<crate::protocol::types::MilkySegment> = message
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                let segments = milky_to_internal_segments(&milky_segments);
                let result = self
                    .service_hub
                    .message
                    .send(
                        &self.core,
                        bot.bound_user_id.clone(),
                        MessageSource::Private {
                            peer_user_id: user_id,
                        },
                        segments,
                        None,
                        Some(bot.bot_id.clone()),
                    )
                    .await?;
                let message_seq = self
                    .service_hub
                    .message
                    .get_seq_by_message_id(&result.id)
                    .await
                    .unwrap_or(0);
                Ok(serde_json::json!({
                    "message_id": result.id,
                    "message_seq": message_seq,
                }))
            }
            "send_group_message" => {
                let group_id = api
                    .params
                    .get("group_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing group_id"))?
                    .to_string();
                let message = api
                    .params
                    .get("message")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| AppError::validation("missing message"))?;
                let milky_segments: Vec<crate::protocol::types::MilkySegment> = message
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                let segments = milky_to_internal_segments(&milky_segments);
                let result = self
                    .service_hub
                    .message
                    .send(
                        &self.core,
                        bot.bound_user_id.clone(),
                        MessageSource::Group { group_id },
                        segments,
                        None,
                        Some(bot.bot_id.clone()),
                    )
                    .await?;
                let message_seq = self
                    .service_hub
                    .message
                    .get_seq_by_message_id(&result.id)
                    .await
                    .unwrap_or(0);
                Ok(serde_json::json!({
                    "message_id": result.id,
                    "message_seq": message_seq,
                }))
            }
            _ => Err(AppError::not_found(format!(
                "unknown api: {}",
                api.api_name
            ))),
        }
    }
}
