use serde::{Deserialize, Serialize};

// ========== Protocol-agnostic API types ==========

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRequest {
    pub api_name: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse {
    pub status: String,
    pub retcode: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ApiResponse {
    pub fn ok(data: impl Serialize) -> Self {
        Self {
            status: "ok".to_string(),
            retcode: 0,
            data: serde_json::to_value(data).ok(),
            message: None,
        }
    }

    pub fn failed(retcode: i32, message: impl Into<String>) -> Self {
        Self {
            status: "failed".to_string(),
            retcode,
            data: None,
            message: Some(message.into()),
        }
    }
}

// ========== Protocol-agnostic event type ==========

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolEvent {
    pub time: u64,
    pub self_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
}

// ========== Bot runtime context ==========

#[derive(Debug, Clone)]
pub struct BotRuntimeContext {
    pub bot_id: String,
    pub bound_user_id: String,
    pub access_token: String,
    #[allow(dead_code)]
    pub listen_addr: std::net::SocketAddr,
}

// ========== Bot configuration ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub version: i32,
    pub protocol: String,
    pub http: HttpConfig,
    pub access_token: String,
    pub event_transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            version: 1,
            protocol: "milky".to_string(),
            http: HttpConfig {
                host: "127.0.0.1".to_string(),
                port: 3001,
            },
            access_token: uuid::Uuid::new_v4().to_string(),
            event_transport: "sse".to_string(),
        }
    }
}

// ========== ProtocolAdapter trait (multi-protocol abstraction) ==========

use crate::error::AppError;
use crate::models::{InternalEvent, UserProfile};

/// Abstraction over protocol format conversion.
///
/// Implementations (e.g. `MilkyAdapter`, future `OneBotAdapter`) convert
/// internal types to protocol-specific representations.  `ProtocolServer`
/// holds `Arc<dyn ProtocolAdapter>` so the active protocol can be switched
/// at runtime.
pub trait ProtocolAdapter: Send + Sync {
    /// Convert an internal error to protocol-specific retcode and message.
    fn adapt_error(&self, err: &AppError) -> (i32, String);

    /// Convert an internal event to a protocol event.
    fn adapt_event(&self, event: &InternalEvent, bot: &BotRuntimeContext) -> Option<ProtocolEvent>;

    /// Convert a user profile to a protocol-specific `get_login_info` response.
    fn adapt_login_info(&self, user: &UserProfile) -> serde_json::Value;

    /// Convert a message-send result to a protocol-specific response.
    fn adapt_message_send(&self, message_id: &str, message_seq: i64) -> serde_json::Value;
}

// ========== Milky 1.2 specific types ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum MilkySegment {
    Text { text: String },
    Image { file: String, url: String },
    At { qq: String },
    AtAll {},
    Face { id: String },
}

// ========== MilkyAdapter ==========

use crate::models::{MessageSegment, MessageSource};

#[derive(Debug, Clone, Default)]
pub struct MilkyAdapter;

impl MilkyAdapter {
    pub fn new() -> Self {
        Self
    }

    fn internal_to_milky_segments(segments: &[MessageSegment]) -> Vec<MilkySegment> {
        crate::protocol::adapter::internal_to_milky_segments(segments)
    }
}

impl ProtocolAdapter for MilkyAdapter {
    fn adapt_error(&self, err: &AppError) -> (i32, String) {
        match err {
            AppError::Validation(msg) => (-400, msg.clone()),
            AppError::NotFound(msg) => (-404, msg.clone()),
            AppError::Conflict(msg) => (-409, msg.clone()),
            AppError::Storage(msg) => (-500, msg.clone()),
            AppError::Internal(msg) => (-500, msg.clone()),
        }
    }

    fn adapt_event(&self, event: &InternalEvent, bot: &BotRuntimeContext) -> Option<ProtocolEvent> {
        match event {
            InternalEvent::Message {
                message_seq,
                sender_user_id,
                source,
                content,
                time,
                ..
            } => {
                let self_id = sender_user_id.parse::<i64>().ok()?;
                let segments = Self::internal_to_milky_segments(content);
                let data = match source {
                    MessageSource::Private { peer_user_id } => {
                        serde_json::json!({
                            "message_type": "private",
                            "user_id": peer_user_id.parse::<i64>().ok()?,
                            "message_seq": message_seq,
                            "message": segments,
                        })
                    }
                    MessageSource::Group { group_id } => {
                        serde_json::json!({
                            "message_type": "group",
                            "group_id": group_id.parse::<i64>().ok()?,
                            "user_id": self_id,
                            "message_seq": message_seq,
                            "message": segments,
                        })
                    }
                };
                Some(ProtocolEvent {
                    time: *time / 1000, // ms -> s
                    self_id: bot.bot_id.clone(),
                    event_type: "message_receive".to_string(),
                    data,
                })
            }
            InternalEvent::FriendRequestCreated {
                request_id,
                initiator_user_id,
                target_user_id,
                time,
            } => {
                let _self_id = target_user_id.parse::<i64>().ok()?;
                let data = serde_json::json!({
                    "request_id": request_id,
                    "user_id": initiator_user_id.parse::<i64>().ok()?,
                    "comment": "",
                });
                Some(ProtocolEvent {
                    time: *time / 1000,
                    self_id: bot.bot_id.clone(),
                    event_type: "friend_request".to_string(),
                    data,
                })
            }
            InternalEvent::GroupRequestCreated {
                request_id,
                group_id,
                request_type: _,
                initiator_user_id,
                target_user_id,
                time,
            } => {
                let _self_id = target_user_id.as_ref()?.parse::<i64>().ok()?;
                let data = serde_json::json!({
                    "request_id": request_id,
                    "group_id": group_id.parse::<i64>().ok()?,
                    "user_id": initiator_user_id.parse::<i64>().ok()?,
                    "comment": "",
                });
                Some(ProtocolEvent {
                    time: *time / 1000,
                    self_id: bot.bot_id.clone(),
                    event_type: "group_join_request".to_string(),
                    data,
                })
            }
            InternalEvent::GroupMemberJoined {
                group_id,
                operator_user_id,
                target_user_id,
                time,
            } => {
                let _self_id = target_user_id.parse::<i64>().ok()?;
                let data = serde_json::json!({
                    "group_id": group_id.parse::<i64>().ok()?,
                    "user_id": target_user_id.parse::<i64>().ok()?,
                    "operator_id": operator_user_id.parse::<i64>().ok()?,
                });
                Some(ProtocolEvent {
                    time: *time / 1000,
                    self_id: bot.bot_id.clone(),
                    event_type: "group_member_increase".to_string(),
                    data,
                })
            }
            InternalEvent::GroupMemberLeft {
                group_id,
                operator_user_id,
                target_user_id,
                time,
            } => {
                let _self_id = target_user_id.parse::<i64>().ok()?;
                let data = serde_json::json!({
                    "group_id": group_id.parse::<i64>().ok()?,
                    "user_id": target_user_id.parse::<i64>().ok()?,
                    "operator_id": operator_user_id.as_ref().and_then(|id| id.parse::<i64>().ok()),
                });
                Some(ProtocolEvent {
                    time: *time / 1000,
                    self_id: bot.bot_id.clone(),
                    event_type: "group_member_decrease".to_string(),
                    data,
                })
            }
            _ => None,
        }
    }

    fn adapt_login_info(&self, user: &UserProfile) -> serde_json::Value {
        serde_json::json!({
            "user_id": user.user_id.parse::<i64>().unwrap_or(0),
            "nickname": user.nickname,
        })
    }

    fn adapt_message_send(&self, message_id: &str, message_seq: i64) -> serde_json::Value {
        serde_json::json!({
            "message_id": message_id,
            "message_seq": message_seq,
        })
    }
}
