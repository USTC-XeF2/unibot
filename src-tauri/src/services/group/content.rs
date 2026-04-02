use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupEventEntity, GroupEventPayload,
    GroupFileEntity, GroupFolderEntity, GroupRole, InternalEvent,
};
use crate::persistence::{GroupEventRecord, NewGroupEventRecord};
use crate::utils::{emit_to_group_members, now_ts};

use super::GroupService;

impl GroupService {
    pub async fn upsert_announcement(
        &self,
        core: &CoreContainer,
        announcement: GroupAnnouncementEntity,
    ) -> AppResult<GroupAnnouncementEntity> {
        core.require_user_context(announcement.sender_user_id)?;

        let sender = self
            .ensure_group_member(announcement.group_id, announcement.sender_user_id)
            .await?;
        if matches!(sender.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can publish group announcements",
            ));
        }

        self.repo.upsert_announcement(&announcement).await?;

        emit_to_group_members(
            core,
            &self.repo,
            announcement.group_id,
            InternalEvent::GroupAnnouncementUpserted {
                announcement_id: announcement.announcement_id.clone(),
                group_id: announcement.group_id,
                sender_user_id: announcement.sender_user_id,
                time: announcement.updated_at,
            },
        )
        .await;

        Ok(announcement)
    }

    pub async fn list_announcements(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Vec<GroupAnnouncementEntity>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .list_announcements(group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn upsert_group_folder(
        &self,
        core: &CoreContainer,
        folder: GroupFolderEntity,
    ) -> AppResult<GroupFolderEntity> {
        core.require_user_context(folder.creator_user_id)?;

        self.ensure_group_member(folder.group_id, folder.creator_user_id)
            .await?;

        self.repo.upsert_group_folder(&folder).await?;

        emit_to_group_members(
            core,
            &self.repo,
            folder.group_id,
            InternalEvent::GroupFolderUpserted {
                folder_id: folder.folder_id.clone(),
                group_id: folder.group_id,
                creator_user_id: folder.creator_user_id,
                time: folder.updated_at,
            },
        )
        .await;

        Ok(folder)
    }

    pub async fn list_group_folders(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Vec<GroupFolderEntity>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .list_group_folders(group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn upsert_group_file(
        &self,
        core: &CoreContainer,
        file: GroupFileEntity,
    ) -> AppResult<GroupFileEntity> {
        core.require_user_context(file.uploader_user_id)?;

        self.ensure_group_member(file.group_id, file.uploader_user_id)
            .await?;

        self.repo.upsert_group_file(&file).await?;

        emit_to_group_members(
            core,
            &self.repo,
            file.group_id,
            InternalEvent::GroupFileUpserted {
                file_id: file.file_id.clone(),
                group_id: file.group_id,
                uploader_user_id: file.uploader_user_id,
                time: file.uploaded_at,
            },
        )
        .await;

        Ok(file)
    }

    pub async fn list_group_files(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Vec<GroupFileEntity>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .list_group_files(group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn set_group_essence_message(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        message_id: i64,
        is_set: bool,
    ) -> AppResult<GroupEssenceMessageEntity> {
        core.require_user_context(user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;

        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can set essence messages",
            ));
        }

        let message = self
            .message_repo
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("message {} not found", message_id)))?;

        if message.source_type != "group" || message.source_id != group_id {
            return Err(AppError::validation(
                "message does not belong to the target group",
            ));
        }

        if message.is_recalled {
            return Err(AppError::validation(
                "recalled message cannot be set as essence",
            ));
        }

        let essence = self
            .repo
            .create_group_essence_message(
                group_id,
                message_id,
                message.sender_user_id,
                user_id,
                is_set,
                now_ts(),
            )
            .await?;

        if essence.is_set {
            self.save_group_event(
                group_id,
                GroupEventPayload::EssenceSet {
                    message_id: essence.message_id,
                    sender_user_id: essence.sender_user_id,
                    operator_user_id: essence.operator_user_id,
                },
                essence.created_at,
            )
            .await?;
        }

        emit_to_group_members(
            core,
            &self.repo,
            group_id,
            InternalEvent::GroupEssenceUpdated {
                essence_id: essence.essence_id,
                group_id: essence.group_id,
                message_id: essence.message_id,
                sender_user_id: essence.sender_user_id,
                operator_user_id: essence.operator_user_id,
                is_set: essence.is_set,
                time: essence.created_at,
            },
        )
        .await;

        Ok(essence)
    }

    pub async fn list_group_essence_messages(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Vec<GroupEssenceMessageEntity>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .list_group_essence_messages(group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn list_group_event_history(
        &self,
        user_id: u64,
        group_id: u64,
        limit: usize,
    ) -> AppResult<Vec<GroupEventEntity>> {
        self.ensure_group_member(group_id, user_id).await?;

        let limit_i64 =
            i64::try_from(limit).map_err(|_| AppError::validation("limit is too large"))?;

        let rows = self.repo.list_group_events(group_id, limit_i64).await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub(super) async fn save_group_event(
        &self,
        group_id: u64,
        payload: GroupEventPayload,
        created_at: u64,
    ) -> AppResult<()> {
        let payload_json = serde_json::to_string(&payload)?;
        self.repo
            .insert_group_event(NewGroupEventRecord {
                group_id,
                payload: payload_json,
                created_at,
            })
            .await?;
        Ok(())
    }
}

impl TryFrom<GroupEventRecord> for GroupEventEntity {
    type Error = AppError;

    fn try_from(row: GroupEventRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            event_id: row.id,
            group_id: row.group_id,
            payload: serde_json::from_str(&row.payload)?,
            created_at: row.created_at,
        })
    }
}
