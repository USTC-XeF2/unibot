use std::path::PathBuf;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupEventEntity,
    GroupEventPayload, GroupFileEntity, GroupFolderEntity, GroupPhotoEntity, GroupRole,
    InternalEvent,
};
use crate::persistence::{GroupEventRecord, NewGroupEventRecord};
use crate::utils::{emit_to_group_members, now_ts};

use super::GroupService;
use super::storage;

impl GroupService {
    pub async fn upsert_announcement(
        &self,
        core: &CoreContainer,
        announcement: GroupAnnouncementEntity,
    ) -> AppResult<GroupAnnouncementEntity> {
        core.require_user_context(&announcement.sender_user_id)?;

        let sender = self
            .ensure_group_member(&announcement.group_id, &announcement.sender_user_id)
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
            &announcement.group_id,
            InternalEvent::GroupAnnouncementUpserted {
                announcement_id: announcement.announcement_id.clone(),
                group_id: announcement.group_id.clone(),
                sender_user_id: announcement.sender_user_id.clone(),
                time: announcement.updated_at,
            },
        )
        .await;

        Ok(announcement)
    }

    pub async fn list_announcements(
        &self,
        user_id: String,
        group_id: String,
    ) -> AppResult<Vec<GroupAnnouncementEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        self.repo
            .list_announcements(&group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn upsert_group_folder(
        &self,
        core: &CoreContainer,
        folder: GroupFolderEntity,
    ) -> AppResult<GroupFolderEntity> {
        core.require_user_context(&folder.creator_user_id)?;

        self.ensure_group_member(&folder.group_id, &folder.creator_user_id)
            .await?;

        self.repo.upsert_group_folder(&folder).await?;

        emit_to_group_members(
            core,
            &self.repo,
            &folder.group_id,
            InternalEvent::GroupFolderUpserted {
                folder_id: folder.folder_id.clone(),
                group_id: folder.group_id.clone(),
                creator_user_id: folder.creator_user_id.clone(),
                time: folder.updated_at,
            },
        )
        .await;

        Ok(folder)
    }

    pub async fn list_group_folders(
        &self,
        user_id: String,
        group_id: String,
    ) -> AppResult<Vec<GroupFolderEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        self.repo
            .list_group_folders(&group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn upsert_group_file(
        &self,
        core: &CoreContainer,
        file: GroupFileEntity,
    ) -> AppResult<GroupFileEntity> {
        core.require_user_context(&file.uploader_user_id)?;

        self.ensure_group_member(&file.group_id, &file.uploader_user_id)
            .await?;

        self.repo.upsert_group_file(&file).await?;

        emit_to_group_members(
            core,
            &self.repo,
            &file.group_id,
            InternalEvent::GroupFileUpserted {
                file_id: file.file_id.clone(),
                group_id: file.group_id.clone(),
                uploader_user_id: file.uploader_user_id.clone(),
                time: file.uploaded_at,
            },
        )
        .await;

        Ok(file)
    }

    pub async fn list_group_files(
        &self,
        user_id: String,
        group_id: String,
        parent_folder_id: Option<String>,
    ) -> AppResult<Vec<GroupFileEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        self.repo
            .list_group_files(&group_id, parent_folder_id.as_deref())
            .await
            .map_err(Into::into)
    }

    pub async fn set_group_essence_message(
        &self,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        message_id: String,
        is_set: bool,
    ) -> AppResult<GroupEssenceMessageEntity> {
        core.require_user_context(&user_id)?;

        let operator = self.ensure_group_member(&group_id, &user_id).await?;

        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can set essence messages",
            ));
        }

        let message = self
            .message_repo
            .get_message_by_id(&message_id)
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
                &group_id,
                &message_id,
                &message.sender_user_id,
                &user_id,
                is_set,
                now_ts(),
            )
            .await?;

        if essence.is_set {
            self.save_group_event(
                &group_id,
                GroupEventPayload::EssenceSet {
                    message_id: essence.message_id.clone(),
                    sender_user_id: essence.sender_user_id.clone(),
                    operator_user_id: essence.operator_user_id.clone(),
                },
                essence.created_at,
            )
            .await?;
        }

        emit_to_group_members(
            core,
            &self.repo,
            &group_id,
            InternalEvent::GroupEssenceUpdated {
                essence_id: essence.essence_id.clone(),
                group_id: essence.group_id.clone(),
                message_id: essence.message_id.clone(),
                sender_user_id: essence.sender_user_id.clone(),
                operator_user_id: essence.operator_user_id.clone(),
                is_set: essence.is_set,
                time: essence.created_at,
            },
        )
        .await;

        Ok(essence)
    }

    pub async fn download_group_file(
        &self,
        user_id: String,
        group_id: String,
        file_id: String,
        app_data_dir: PathBuf,
    ) -> AppResult<String> {
        self.ensure_group_member(&group_id, &user_id).await?;

        let file = self
            .repo
            .get_group_file_by_id(&file_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("file {} not found", file_id)))?;

        if file.group_id != group_id {
            return Err(AppError::validation(
                "file does not belong to the target group",
            ));
        }

        let file_path = file
            .file_path
            .ok_or_else(|| AppError::not_found("file has no local path"))?;

        let abs_path = storage::validate_group_file_path(&file_path, &app_data_dir).await?;

        Ok(abs_path.to_string_lossy().to_string())
    }

    pub async fn delete_group_file(
        &self,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        file_id: String,
        app_data_dir: PathBuf,
    ) -> AppResult<()> {
        core.require_user_context(&user_id)?;

        let operator = self.ensure_group_member(&group_id, &user_id).await?;

        let file = self
            .repo
            .get_group_file_by_id(&file_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("file {} not found", file_id)))?;

        if file.group_id != group_id {
            return Err(AppError::validation(
                "file does not belong to the target group",
            ));
        }

        if file.uploader_user_id != user_id
            && !matches!(operator.role, GroupRole::Owner | GroupRole::Admin)
        {
            return Err(AppError::validation(
                "only uploader or owner/admin can delete files",
            ));
        }

        self.repo.delete_group_file(&file_id).await?;

        if let Some(ref file_path) = file.file_path {
            storage::delete_group_file_disk(file_path, &app_data_dir).await;
        }

        Ok(())
    }

    pub async fn create_group_album(
        &self,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        name: String,
    ) -> AppResult<GroupAlbumEntity> {
        core.require_user_context(&user_id)?;
        self.ensure_group_member(&group_id, &user_id).await?;

        let album = GroupAlbumEntity {
            album_id: crate::utils::new_db_id(),
            group_id,
            name,
            cover_url: None,
            photo_count: 0,
            created_at: now_ts(),
            updated_at: now_ts(),
        };

        self.repo.create_group_album(&album).await?;

        emit_to_group_members(
            core,
            &self.repo,
            &album.group_id,
            InternalEvent::GroupAlbumCreated {
                album_id: album.album_id.clone(),
                group_id: album.group_id.clone(),
                name: album.name.clone(),
                time: album.created_at,
            },
        )
        .await;

        Ok(album)
    }

    pub async fn list_group_albums(
        &self,
        user_id: String,
        group_id: String,
    ) -> AppResult<Vec<GroupAlbumEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        self.repo
            .list_group_albums(&group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_group_album(
        &self,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        album_id: String,
        app_data_dir: PathBuf,
    ) -> AppResult<()> {
        core.require_user_context(&user_id)?;

        let operator = self.ensure_group_member(&group_id, &user_id).await?;
        if !matches!(operator.role, GroupRole::Owner | GroupRole::Admin) {
            return Err(AppError::validation("only owner/admin can delete albums"));
        }

        let photos = self.repo.list_group_photos(&album_id).await?;
        for photo in &photos {
            if let Some(ref file_path) = photo.file_path {
                storage::delete_group_file_disk(file_path, &app_data_dir).await;
            }
        }

        self.repo.delete_group_album(&album_id).await?;

        emit_to_group_members(
            core,
            &self.repo,
            &group_id,
            InternalEvent::GroupAlbumDeleted {
                album_id: album_id.clone(),
                group_id: group_id.clone(),
                time: now_ts(),
            },
        )
        .await;

        Ok(())
    }

    pub async fn upload_group_photo(
        &self,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        album_id: String,
        photo_id: String,
        source_path: String,
        description: Option<String>,
        app_data_dir: PathBuf,
    ) -> AppResult<GroupPhotoEntity> {
        core.require_user_context(&user_id)?;
        self.ensure_group_member(&group_id, &user_id).await?;

        let src = std::path::Path::new(&source_path);
        let file_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&photo_id)
            .to_string();

        let file_path =
            storage::copy_file_to_groups_dir(src, &group_id, &photo_id, &file_name, &app_data_dir)
                .await?;

        let metadata = tokio::fs::metadata(&app_data_dir.join(&file_path))
            .await
            .map_err(|e| AppError::storage(format!("failed to get photo metadata: {e}")))?;

        let photo = GroupPhotoEntity {
            photo_id,
            album_id,
            group_id: group_id.clone(),
            url: file_path.clone(),
            file_path: Some(file_path),
            description,
            uploader_user_id: user_id,
            file_size: Some(metadata.len()),
            created_at: now_ts(),
        };

        self.repo.create_group_photo(&photo).await?;

        emit_to_group_members(
            core,
            &self.repo,
            &group_id,
            InternalEvent::GroupPhotoUploaded {
                photo_id: photo.photo_id.clone(),
                album_id: photo.album_id.clone(),
                group_id: photo.group_id.clone(),
                time: photo.created_at,
            },
        )
        .await;

        Ok(photo)
    }

    pub async fn list_group_photos(
        &self,
        user_id: String,
        group_id: String,
        album_id: String,
    ) -> AppResult<Vec<GroupPhotoEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        self.repo
            .list_group_photos(&album_id)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_group_photo(
        &self,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        photo_id: String,
        app_data_dir: PathBuf,
    ) -> AppResult<()> {
        core.require_user_context(&user_id)?;

        let operator = self.ensure_group_member(&group_id, &user_id).await?;

        let photo = self
            .repo
            .get_group_photo_by_id(&photo_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("photo {} not found", photo_id)))?;

        if photo.group_id != group_id {
            return Err(AppError::validation(
                "photo does not belong to the target group",
            ));
        }

        if photo.uploader_user_id != user_id
            && !matches!(operator.role, GroupRole::Owner | GroupRole::Admin)
        {
            return Err(AppError::validation(
                "only uploader or owner/admin can delete photos",
            ));
        }

        self.repo.delete_group_photo(&photo_id).await?;

        if let Some(ref file_path) = photo.file_path {
            storage::delete_group_file_disk(file_path, &app_data_dir).await;
        }

        Ok(())
    }

    pub async fn list_group_essence_messages(
        &self,
        user_id: String,
        group_id: String,
    ) -> AppResult<Vec<GroupEssenceMessageEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        self.repo
            .list_group_essence_messages(&group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn list_group_event_history(
        &self,
        user_id: String,
        group_id: String,
        limit: usize,
    ) -> AppResult<Vec<GroupEventEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;

        let limit_i64 =
            i64::try_from(limit).map_err(|_| AppError::validation("limit is too large"))?;

        let rows = self.repo.list_group_events(&group_id, limit_i64).await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub(super) async fn save_group_event(
        &self,
        group_id: &str,
        payload: GroupEventPayload,
        created_at: u64,
    ) -> AppResult<()> {
        let payload_json = serde_json::to_string(&payload)?;
        self.repo
            .insert_group_event(NewGroupEventRecord {
                group_id: group_id.to_string(),
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
