use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{InternalEvent, MessageSource};
use crate::protocol::adapter::milky_to_internal_segments;
use crate::protocol::types::{ApiRequest, BotRuntimeContext, ProtocolAdapter};
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
    adapter: Arc<dyn ProtocolAdapter>,
}

impl VirtualBackend {
    pub fn new(
        service_hub: ServiceHub,
        core: Arc<CoreContainer>,
        adapter: Arc<dyn ProtocolAdapter>,
    ) -> Self {
        Self {
            service_hub,
            core,
            adapter,
        }
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
                Ok(self.adapter.adapt_login_info(&user))
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
                Ok(self.adapter.adapt_message_send(&result.id, message_seq))
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
                Ok(self.adapter.adapt_message_send(&result.id, message_seq))
            }
            "get_friend_list" => {
                let friend_ids = self
                    .service_hub
                    .user
                    .list_friends(bot.bound_user_id.clone())
                    .await?;
                let mut data = Vec::new();
                for friend_id in friend_ids {
                    if let Some(profile) = self.service_hub.user.get_user_by_id(&friend_id).await? {
                        data.push(serde_json::json!({
                            "user_id": profile.user_id.parse::<i64>().unwrap_or(0),
                            "nickname": profile.nickname,
                            "remark": "",
                        }));
                    }
                }
                Ok(serde_json::json!({ "data": data }))
            }
            "get_group_list" => {
                let groups = self
                    .service_hub
                    .group
                    .list_user_groups(&self.core, bot.bound_user_id.clone())
                    .await?;
                let data: Vec<serde_json::Value> = groups
                    .into_iter()
                    .map(|g| {
                        serde_json::json!({
                            "group_id": g.group_id.parse::<i64>().unwrap_or(0),
                            "group_name": g.group_name,
                            "member_count": g.member_count,
                            "max_member_count": g.max_member_count,
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "data": data }))
            }
            "get_group_info" => {
                let group_id = api
                    .params
                    .get("group_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing group_id"))?
                    .to_string();
                let group = self
                    .service_hub
                    .group
                    .get_group(&group_id)
                    .await?
                    .ok_or_else(|| AppError::not_found("group not found"))?;
                Ok(serde_json::json!({
                    "data": {
                        "group_id": group.group_id.parse::<i64>().unwrap_or(0),
                        "group_name": group.group_name,
                        "member_count": group.member_count,
                        "max_member_count": group.max_member_count,
                    }
                }))
            }
            "get_group_member_list" => {
                let group_id = api
                    .params
                    .get("group_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| AppError::validation("missing group_id"))?
                    .to_string();
                let members = self
                    .service_hub
                    .group
                    .list_group_members(bot.bound_user_id.clone(), group_id)
                    .await?;
                let data: Vec<serde_json::Value> = members
                    .into_iter()
                    .map(|m| {
                        serde_json::json!({
                            "user_id": m.user_id.parse::<i64>().unwrap_or(0),
                            "nickname": m.card,
                            "card": m.card,
                            "role": match m.role {
                                crate::models::GroupRole::Owner => "owner",
                                crate::models::GroupRole::Admin => "admin",
                                crate::models::GroupRole::Member => "member",
                            },
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "data": data }))
            }
            _ => Err(AppError::not_found(format!(
                "unknown api: {}",
                api.api_name
            ))),
        }
    }
}
