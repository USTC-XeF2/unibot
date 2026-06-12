use serde::{Deserialize, Serialize};

use crate::models::InternalEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevToolsEvent {
    pub event: InternalEvent,
}
