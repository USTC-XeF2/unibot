use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupCategoryEntity, GroupEssenceMessageEntity,
    GroupFileEntity, GroupFolderEntity, GroupMemberProfile, GroupPhotoEntity, GroupProfile,
    GroupRequestEntity, GroupWholeMuteState,
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
pub(super) struct UserGroupRow {
    pub group_id: String,
    pub group_name: String,
    pub owner_user_id: String,
    pub member_count: u32,
    pub max_member_count: u32,
    pub group_status: String,
    pub category_id: Option<String>,
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
    pub file_path: Option<String>,
    pub download_count: u32,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupAlbumRow {
    pub album_id: String,
    pub group_id: String,
    pub name: String,
    pub cover_url: Option<String>,
    pub photo_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(sqlx::FromRow)]
pub(super) struct GroupPhotoRow {
    pub photo_id: String,
    pub album_id: String,
    pub group_id: String,
    pub url: String,
    pub file_path: Option<String>,
    pub description: Option<String>,
    pub uploader_user_id: String,
    pub file_size: Option<i64>,
    pub created_at: i64,
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

#[derive(sqlx::FromRow)]
pub(super) struct GroupCategoryRow {
    pub category_id: String,
    pub owner_user_id: String,
    pub name: String,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

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
            category_id: None,
        })
    }
}

impl TryFrom<UserGroupRow> for GroupProfile {
    type Error = sqlx::Error;

    fn try_from(row: UserGroupRow) -> Result<Self, Self::Error> {
        Ok(Self {
            group_id: row.group_id,
            group_name: row.group_name,
            owner_user_id: row.owner_user_id,
            member_count: row.member_count,
            max_member_count: row.max_member_count,
            group_status: codecs::group_status_from_db(&row.group_status)?,
            category_id: row.category_id,
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
            download_count: row.download_count,
            file_path: row.file_path,
        })
    }
}

impl TryFrom<GroupAlbumRow> for GroupAlbumEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupAlbumRow) -> Result<Self, Self::Error> {
        Ok(Self {
            album_id: row.album_id,
            group_id: row.group_id,
            name: row.name,
            cover_url: row.cover_url,
            photo_count: row.photo_count,
            created_at: row.created_at as u64,
            updated_at: row.updated_at as u64,
        })
    }
}

impl TryFrom<GroupPhotoRow> for GroupPhotoEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupPhotoRow) -> Result<Self, Self::Error> {
        Ok(Self {
            photo_id: row.photo_id,
            album_id: row.album_id,
            group_id: row.group_id,
            url: row.url,
            file_path: row.file_path,
            description: row.description,
            uploader_user_id: row.uploader_user_id,
            file_size: row.file_size.map(|s| s as u64),
            created_at: row.created_at as u64,
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

impl TryFrom<GroupCategoryRow> for GroupCategoryEntity {
    type Error = sqlx::Error;

    fn try_from(row: GroupCategoryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            category_id: row.category_id,
            owner_user_id: row.owner_user_id,
            name: row.name,
            sort_order: row.sort_order,
            created_at: row.created_at as u64,
            updated_at: row.updated_at as u64,
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
