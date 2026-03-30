use crate::models::{
    GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity, GroupFolderEntity,
    GroupMemberProfile, GroupProfile, GroupRequestEntity, GroupRequestType, GroupRole,
    GroupWholeMuteState, RequestState,
};

#[derive(sqlx::FromRow)]
pub(super) struct GroupRow {
    pub group_id: u64,
    pub group_name: String,
    pub owner_user_id: u64,
    pub member_count: u32,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupMemberRow {
    pub group_id: u64,
    pub user_id: u64,
    pub card: String,
    pub title: String,
    pub role: GroupRole,
    pub joined_at: u64,
    pub last_sent_at: u64,
    pub mute_until: Option<u64>,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupRequestRow {
    pub id: i64,
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

#[derive(sqlx::FromRow)]
pub(super) struct GroupWholeMuteRow {
    pub group_id: u64,
    pub muted: bool,
    pub mute_until: Option<u64>,
    pub operator_user_id: Option<u64>,
    pub updated_at: u64,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupAnnouncementRow {
    pub announcement_id: String,
    pub group_id: u64,
    pub sender_user_id: u64,
    pub content: String,
    pub image_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupFileRow {
    pub file_id: String,
    pub group_id: u64,
    pub parent_folder_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: Option<String>,
    pub uploader_user_id: u64,
    pub uploaded_at: u64,
    pub expire_at: Option<u64>,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupFolderRow {
    pub folder_id: String,
    pub group_id: u64,
    pub parent_folder_id: String,
    pub folder_name: String,
    pub creator_user_id: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub file_count: u32,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupEssenceRow {
    pub id: i64,
    pub group_id: u64,
    pub message_id: i64,
    pub sender_user_id: u64,
    pub operator_user_id: u64,
    pub is_set: bool,
    pub created_at: u64,
}

pub(super) const DEFAULT_MAX_MEMBER_COUNT: u32 = 500;

impl From<GroupRow> for GroupProfile {
    fn from(row: GroupRow) -> Self {
        Self {
            group_id: row.group_id,
            group_name: row.group_name,
            owner_user_id: row.owner_user_id,
            member_count: row.member_count,
            max_member_count: DEFAULT_MAX_MEMBER_COUNT,
        }
    }
}

impl From<GroupMemberRow> for GroupMemberProfile {
    fn from(row: GroupMemberRow) -> Self {
        Self {
            group_id: row.group_id,
            user_id: row.user_id,
            card: row.card,
            title: row.title,
            role: row.role,
            joined_at: row.joined_at,
            last_sent_at: row.last_sent_at,
            mute_until: row.mute_until,
        }
    }
}

impl From<GroupRequestRow> for GroupRequestEntity {
    fn from(row: GroupRequestRow) -> Self {
        Self {
            request_id: row.id,
            group_id: row.group_id,
            request_type: row.request_type,
            initiator_user_id: row.initiator_user_id,
            target_user_id: row.target_user_id,
            comment: row.comment,
            state: row.state,
            created_at: row.created_at,
            handled_at: row.handled_at,
            operator_user_id: row.operator_user_id,
        }
    }
}

impl From<GroupWholeMuteRow> for GroupWholeMuteState {
    fn from(row: GroupWholeMuteRow) -> Self {
        Self {
            group_id: row.group_id,
            muted: row.muted,
            mute_until: row.mute_until,
            operator_user_id: row.operator_user_id,
            updated_at: row.updated_at,
        }
    }
}

impl From<GroupAnnouncementRow> for GroupAnnouncementEntity {
    fn from(row: GroupAnnouncementRow) -> Self {
        Self {
            announcement_id: row.announcement_id,
            group_id: row.group_id,
            sender_user_id: row.sender_user_id,
            content: row.content,
            image_url: row.image_url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<GroupFileRow> for GroupFileEntity {
    fn from(row: GroupFileRow) -> Self {
        Self {
            file_id: row.file_id,
            group_id: row.group_id,
            parent_folder_id: row.parent_folder_id,
            file_name: row.file_name,
            file_size: row.file_size,
            file_hash: row.file_hash,
            uploader_user_id: row.uploader_user_id,
            uploaded_at: row.uploaded_at,
            expire_at: row.expire_at,
            download_count: 0,
        }
    }
}

impl From<GroupFolderRow> for GroupFolderEntity {
    fn from(row: GroupFolderRow) -> Self {
        Self {
            folder_id: row.folder_id,
            group_id: row.group_id,
            parent_folder_id: row.parent_folder_id,
            folder_name: row.folder_name,
            creator_user_id: row.creator_user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            file_count: row.file_count,
        }
    }
}

impl From<GroupEssenceRow> for GroupEssenceMessageEntity {
    fn from(row: GroupEssenceRow) -> Self {
        Self {
            essence_id: row.id,
            group_id: row.group_id,
            message_id: row.message_id,
            sender_user_id: row.sender_user_id,
            operator_user_id: row.operator_user_id,
            is_set: row.is_set,
            created_at: row.created_at,
        }
    }
}
