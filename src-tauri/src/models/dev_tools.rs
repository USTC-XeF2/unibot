use serde::{Deserialize, Serialize};

use crate::models::InternalEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevToolsEvent {
    pub recipient_user_id: String,
    pub event: InternalEvent,
}
