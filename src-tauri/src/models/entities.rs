use serde::{Deserialize, Serialize};

use super::internal::MessageSegment;

pub type DbId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Disabled,
    Unavailable,
    Deleted,
}

impl Default for AccountStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
    Active,
    Dissolved,
    Unavailable,
}

impl Default for GroupStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserProfile {
    pub user_id: DbId,
    pub nickname: String,
    pub avatar: String,
    pub signature: String,
    #[serde(default)]
    pub account_status: AccountStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupProfile {
    pub group_id: DbId,
    pub group_name: String,
    pub owner_user_id: DbId,
    #[serde(default)]
    pub member_count: u32,
    pub max_member_count: u32,
    #[serde(default)]
    pub group_status: GroupStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMemberProfile {
    pub group_id: DbId,
    pub user_id: DbId,
    pub card: String,
    pub title: String,
    pub role: GroupRole,
    pub joined_at: u64,
    pub last_sent_at: u64,
    pub mute_until: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "scene", rename_all = "snake_case")]
pub enum MessageSource {
    Private { peer_user_id: DbId },
    Group { group_id: DbId },
}

impl MessageSource {
    pub fn to_db_parts(&self) -> (&'static str, &str) {
        match self {
            MessageSource::Private { peer_user_id } => ("private", peer_user_id.as_str()),
            MessageSource::Group { group_id } => ("group", group_id.as_str()),
        }
    }
}

impl TryFrom<(&str, String)> for MessageSource {
    type Error = String;

    fn try_from(value: (&str, String)) -> Result<Self, Self::Error> {
        let (source_type, source_id) = value;
        match source_type {
            "private" => Ok(MessageSource::Private {
                peer_user_id: source_id,
            }),
            "group" => Ok(MessageSource::Group {
                group_id: source_id,
            }),
            _ => Err(format!("unknown source type: {source_type}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageRecallInfo {
    pub recalled: bool,
    pub recalled_by_user_id: Option<DbId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageEntity {
    pub message_id: DbId,
    pub sender_user_id: DbId,
    pub source: MessageSource,
    pub content: Vec<MessageSegment>,
    pub quoted_message_id: Option<DbId>,
    pub created_at: u64,
    pub recall: MessageRecallInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageReactionEntity {
    pub reaction_id: DbId,
    pub message_id: DbId,
    pub source: MessageSource,
    pub operator_user_id: DbId,
    pub face_id: String,
    pub is_add: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PokeEntity {
    pub poke_id: DbId,
    pub source: MessageSource,
    pub sender_user_id: DbId,
    pub target_user_id: DbId,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupAnnouncementEntity {
    pub announcement_id: String,
    pub group_id: DbId,
    pub sender_user_id: DbId,
    pub content: String,
    pub image_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupFileEntity {
    pub file_id: String,
    pub group_id: DbId,
    pub parent_folder_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: Option<String>,
    pub uploader_user_id: DbId,
    pub uploaded_at: u64,
    pub expire_at: Option<u64>,
    pub download_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupFolderEntity {
    pub folder_id: String,
    pub group_id: DbId,
    pub parent_folder_id: String,
    pub folder_name: String,
    pub creator_user_id: DbId,
    pub created_at: u64,
    pub updated_at: u64,
    pub file_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    Pending,
    Accepted,
    Rejected,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FriendRequestEntity {
    pub request_id: DbId,
    pub initiator_user_id: DbId,
    pub target_user_id: DbId,
    pub comment: String,
    pub state: RequestState,
    pub created_at: u64,
    pub handled_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupRequestType {
    Join,
    Invite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupRequestEntity {
    pub request_id: DbId,
    pub group_id: DbId,
    pub request_type: GroupRequestType,
    pub initiator_user_id: DbId,
    pub target_user_id: Option<DbId>,
    pub comment: Option<String>,
    pub state: RequestState,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<DbId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupWholeMuteState {
    pub group_id: DbId,
    pub muted: bool,
    pub mute_until: Option<u64>,
    pub operator_user_id: Option<DbId>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupEssenceMessageEntity {
    pub essence_id: DbId,
    pub group_id: DbId,
    pub message_id: DbId,
    pub sender_user_id: DbId,
    pub operator_user_id: DbId,
    pub is_set: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GroupEventPayload {
    MemberJoined {
        operator_user_id: DbId,
        joined_user_id: DbId,
    },
    MemberMuted {
        operator_user_id: DbId,
        target_user_id: DbId,
        mute_until: Option<u64>,
    },
    EssenceSet {
        message_id: DbId,
        sender_user_id: DbId,
        operator_user_id: DbId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupEventEntity {
    pub event_id: DbId,
    pub group_id: DbId,
    pub payload: GroupEventPayload,
    pub created_at: u64,
}
