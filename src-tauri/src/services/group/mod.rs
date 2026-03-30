use serde::Serialize;

use crate::persistence::{GroupRepo, MessageRepo};

mod basic;
mod content;
mod management;
mod requests;

#[derive(Debug, Clone, Serialize)]
pub struct MuteGroupMemberResult {
    pub group_id: u64,
    pub target_user_id: u64,
    pub muted: bool,
    pub mute_until: Option<u64>,
}

#[derive(Clone)]
pub struct GroupService {
    repo: GroupRepo,
    message_repo: MessageRepo,
}

impl GroupService {
    pub fn new(repo: GroupRepo, message_repo: MessageRepo) -> Self {
        Self { repo, message_repo }
    }
}
