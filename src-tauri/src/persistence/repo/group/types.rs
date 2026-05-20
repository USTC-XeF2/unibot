use crate::models::{
    GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupFileEntity, GroupFolderEntity,
    GroupMemberProfile, GroupProfile, GroupRequestEntity, GroupRequestType, GroupRole,
    GroupWholeMuteState, RequestState,
};
use crate::persistence::repo::codecs;

#[derive(sqlx::FromRow)]
pub(super) struct GroupRow {
    pub group_id: String,
    pub group_name: String,
    pub owner_user_id: String,
    pub member_count: u32,
    pub max_member_count: u32,
    pub group_status: String,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupMemberRow {
    pub group_id: String,
    pub user_id: String,
    pub card: String,
    pub special_title: String,
    pub role: String,
    pub joined_at: u64,
    pub last_sent_at: u64,
    pub mute_until: Option<u64>,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupRequestRow {
    pub id: String,
    pub group_id: String,
    pub request_type: String,
    pub initiator_user_id: String,
    pub target_user_id: Option<String>,
    pub comment: Option<String>,
    pub state: String,
    pub created_at: u64,
    pub handled_at: Option<u64>,
    pub operator_user_id: Option<String>,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupWholeMuteRow {
    pub group_id: String,
    pub muted: bool,
    pub mute_until: Option<u64>,
    pub operator_user_id: Option<String>,
    pub updated_at: u64,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupAnnouncementRow {
    pub announcement_id: String,
    pub group_id: String,
    pub sender_user_id: String,
    pub content: String,
    pub image_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupFileRow {
    pub file_id: String,
    pub group_id: String,
    pub parent_folder_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: Option<String>,
    pub uploader_user_id: String,
    pub uploaded_at: u64,
    pub expire_at: Option<u64>,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupFolderRow {
    pub folder_id: String,
    pub group_id: String,
    pub parent_folder_id: String,
    pub folder_name: String,
    pub creator_user_id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub file_count: u32,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupEssenceRow {
    pub id: String,
    pub group_id: String,
    pub message_id: String,
    pub sender_user_id: String,
    pub operator_user_id: String,
    pub is_set: bool,
    pub created_at: u64,
}

pub(super) const DEFAULT_MAX_MEMBER_COUNT: u32 = 500;

impl TryFrom<GroupRow> for GroupProfile {
    type Error = sqlx::Error;

    fn try_from(row: GroupRow) -> Result<Self, Self::Error> {
        Ok(Self {
            group_id: row.group_id,
            group_name: row.group_name,
            owner_user_id: row.owner_user_id,
            member_count: row.member_count,
            max_member_count: row.max_member_count,
            group_status: codecs::group_status_from_db(&row.group_status)?,
        })
    }
}

impl TryFrom<GroupMemberRow> for GroupMemberProfile {
    type Error = sqlx::Error;

    fn try_from(row: GroupMemberRow) -> Result<Self, Self::Error> {
        Ok(Self {
            group_id: row.group_id,
            user_id: row.user_id,
            card: row.card,
            title: row.special_title,
            role: codecs::group_role_from_db(&row.role)?,
            joined_at: row.joined_at,
            last_sent_at: row.last_sent_at,
            mute_until: row.mute_until,
        })
    }
}

impl TryFrom<GroupRequestRow> for GroupRequestEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: row.id,
            group_id: row.group_id,
            request_type: codecs::group_request_type_from_db(&row.request_type)?,
            initiator_user_id: row.initiator_user_id,
            target_user_id: row.target_user_id,
            comment: row.comment,
            state: codecs::request_state_from_db(&row.state)?,
            created_at: row.created_at,
            handled_at: row.handled_at,
            operator_user_id: row.operator_user_id,
        })
    }
}

impl TryFrom<GroupWholeMuteRow> for GroupWholeMuteState {
    type Error = sqlx::Error;

    fn try_from(row: GroupWholeMuteRow) -> Result<Self, Self::Error> {
        Ok(Self {
            group_id: row.group_id,
            muted: row.muted,
            mute_until: row.mute_until,
            operator_user_id: row.operator_user_id,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<GroupAnnouncementRow> for GroupAnnouncementEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupAnnouncementRow) -> Result<Self, Self::Error> {
        Ok(Self {
            announcement_id: row.announcement_id,
            group_id: row.group_id,
            sender_user_id: row.sender_user_id,
            content: row.content,
            image_url: row.image_url,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<GroupFileRow> for GroupFileEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupFileRow) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

impl TryFrom<GroupFolderRow> for GroupFolderEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupFolderRow) -> Result<Self, Self::Error> {
        Ok(Self {
            folder_id: row.folder_id,
            group_id: row.group_id,
            parent_folder_id: row.parent_folder_id,
            folder_name: row.folder_name,
            creator_user_id: row.creator_user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            file_count: row.file_count,
        })
    }
}

impl TryFrom<GroupEssenceRow> for GroupEssenceMessageEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupEssenceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            essence_id: row.id,
            group_id: row.group_id,
            message_id: row.message_id,
            sender_user_id: row.sender_user_id,
            operator_user_id: row.operator_user_id,
            is_set: row.is_set,
            created_at: row.created_at,
        })
    }
}