use serde::{Deserialize, Serialize};

use super::internal::MessageSegment;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserProfile {
    pub user_id: u64,
    pub nickname: String,
    pub avatar: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupProfile {
    pub group_id: u64,
    pub group_name: String,
    pub owner_user_id: u64,
    #[serde(default)]
    pub member_count: u32,
    pub max_member_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[repr(i64)]
#[sqlx(type_name = "INTEGER")]
pub enum GroupRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMemberProfile {
    pub group_id: u64,
    pub user_id: u64,
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
    Private { peer_user_id: u64 },
    Group { group_id: u64 },
}

impl MessageSource {
    pub fn to_db_parts(&self) -> (&'static str, u64) {
        match self {
            MessageSource::Private { peer_user_id } => ("private", *peer_user_id),
            MessageSource::Group { group_id } => ("group", *group_id),
        }
    }
}

impl TryFrom<(&str, u64)> for MessageSource {
    type Error = String;

    fn try_from(value: (&str, u64)) -> Result<Self, Self::Error> {
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
    pub recalled_by_user_id: Option<u64>,
    pub recalled_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageEntity {
    pub message_id: i64,
    pub sender_user_id: u64,
    pub source: MessageSource,
    pub content: Vec<MessageSegment>,
    pub created_at: u64,
    pub recall: MessageRecallInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageReactionEntity {
    pub reaction_id: i64,
    pub message_id: i64,
    pub source: MessageSource,
    pub operator_user_id: u64,
    pub face_id: String,
    pub is_add: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PokeEntity {
    pub poke_id: i64,
    pub source: MessageSource,
    pub sender_user_id: u64,
    pub target_user_id: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupAnnouncementEntity {
    pub announcement_id: String,
    pub group_id: u64,
    pub sender_user_id: u64,
    pub content: String,
    pub image_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupFileEntity {
    pub file_id: String,
    pub group_id: u64,
    pub parent_folder_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: Option<String>,
    pub uploader_user_id: u64,
    pub uploaded_at: u64,
    pub expire_at: Option<u64>,
    pub download_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupFolderEntity {
    pub folder_id: String,
    pub group_id: u64,
    pub parent_folder_id: String,
    pub folder_name: String,
    pub creator_user_id: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub file_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[repr(i64)]
#[sqlx(type_name = "INTEGER")]
pub enum RequestState {
    Pending,
    Accepted,
    Rejected,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FriendRequestEntity {
    pub request_id: i64,
    pub initiator_user_id: u64,
    pub target_user_id: u64,
    pub comment: String,
    pub state: RequestState,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[repr(i64)]
#[sqlx(type_name = "INTEGER")]
pub enum GroupRequestType {
    Join,
    Invite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupRequestEntity {
    pub request_id: i64,
    pub group_id: u64,
    pub request_type: GroupRequestType,
    pub initiator_user_id: u64,
    pub target_user_id: Option<u64>,
    pub comment: Option<String>,
    pub state: RequestState,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupWholeMuteState {
    pub group_id: u64,
    pub muted: bool,
    pub mute_until: Option<u64>,
    pub operator_user_id: Option<u64>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupEssenceMessageEntity {
    pub essence_id: i64,
    pub group_id: u64,
    pub message_id: i64,
    pub sender_user_id: u64,
    pub operator_user_id: u64,
    pub is_set: bool,
    pub created_at: u64,
}
