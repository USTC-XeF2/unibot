use std::path::PathBuf;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupEssenceMessageEntity, GroupEventEntity,
    GroupEventPayload, GroupFileEntity, GroupFolderEntity, GroupPhotoEntity, GroupRole,
    InternalEvent,
};
use crate::persistence::{GroupEventRecord, NewGroupEventRecord};
use crate::utils::{emit_group_content_to_windows, emit_to_group_members, now_ts};

use super::GroupService;
use super::storage;

fn prepare_announcement_for_upsert(
    mut announcement: GroupAnnouncementEntity,
) -> GroupAnnouncementEntity {
    let now = crate::utils::now_ts();
    if announcement.announcement_id.trim().is_empty() {
        announcement.announcement_id = crate::utils::new_db_id();
        announcement.created_at = now;
    }
    announcement.updated_at = now;
    announcement
}

fn prepare_folder_for_upsert(mut folder: GroupFolderEntity) -> GroupFolderEntity {
    let now = crate::utils::now_ts();
    if folder.folder_id.trim().is_empty() {
        folder.folder_id = crate::utils::new_db_id();
        folder.created_at = now;
    }
    folder.updated_at = now;
    folder
}

/// Resolve a stored relative media path (e.g. `groups/<gid>/files/<name>`) into
/// an absolute path string for the frontend's `convertFileSrc`. Returns `None`
/// when the input is `None` or empty. Absolute paths are passed through so we
/// stay tolerant of any legacy rows that still hold an absolute path.
fn resolve_relative_url(stored: Option<&str>, app_data_dir: &std::path::Path) -> Option<String> {
    let value = stored?;
    if value.is_empty() {
        return None;
    }
    let path = std::path::Path::new(value);
    if path.is_absolute() {
        return Some(value.to_string());
    }
    Some(app_data_dir.join(path).to_string_lossy().to_string())
}

fn ensure_album_belongs_to_group(album: &GroupAlbumEntity, group_id: &str) -> AppResult<()> {
    if album.group_id != group_id {
        return Err(AppError::validation(
            "album does not belong to the target group",
        ));
    }
    Ok(())
}

fn ensure_parent_folder_belongs_to_group(
    folder: &GroupFolderEntity,
    group_id: &str,
) -> AppResult<()> {
    if folder.group_id != group_id {
        return Err(AppError::validation(
            "parent folder does not belong to the target group",
        ));
    }
    Ok(())
}

impl GroupService {
    pub async fn upsert_announcement(
        &self,
        app: &tauri::AppHandle,
        core: &CoreContainer,
        announcement: GroupAnnouncementEntity,
    ) -> AppResult<GroupAnnouncementEntity> {
        let announcement = prepare_announcement_for_upsert(announcement);

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

        let event = InternalEvent::GroupAnnouncementUpserted {
            announcement_id: announcement.announcement_id.clone(),
            group_id: announcement.group_id.clone(),
            sender_user_id: announcement.sender_user_id.clone(),
            time: announcement.updated_at,
        };
        emit_to_group_members(core, &self.repo, &announcement.group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &announcement.group_id, &event).await?;

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

    pub async fn delete_announcement(
        &self,
        app: &tauri::AppHandle,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        announcement_id: String,
    ) -> AppResult<()> {
        core.require_user_context(&user_id)?;

        let operator = self.ensure_group_member(&group_id, &user_id).await?;
        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can delete group announcements",
            ));
        }

        self.repo
            .delete_announcement_or_not_found(&group_id, &announcement_id)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => {
                    AppError::not_found(format!("announcement {} not found", announcement_id))
                }
                err => err.into(),
            })?;

        let event = InternalEvent::GroupAnnouncementDeleted {
            announcement_id: announcement_id.clone(),
            group_id: group_id.clone(),
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

        Ok(())
    }

    pub async fn validate_group_folder_upsert(
        &self,
        core: &CoreContainer,
        folder: &GroupFolderEntity,
    ) -> AppResult<()> {
        core.require_user_context(&folder.creator_user_id)?;

        let operator = self
            .ensure_group_member(&folder.group_id, &folder.creator_user_id)
            .await?;

        if let Some(parent_folder_id) = folder.parent_folder_id.as_deref() {
            let parent = self
                .repo
                .get_group_folder_by_id(parent_folder_id)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("folder {} not found", parent_folder_id))
                })?;
            ensure_parent_folder_belongs_to_group(&parent, &folder.group_id)?;
        }

        if let Some(existing) = self.repo.get_group_folder_by_id(&folder.folder_id).await? {
            if existing.group_id != folder.group_id {
                return Err(AppError::validation(
                    "folder does not belong to the target group",
                ));
            }

            if existing.creator_user_id != folder.creator_user_id
                && !matches!(operator.role, GroupRole::Owner | GroupRole::Admin)
            {
                return Err(AppError::validation(
                    "only owner/admin or creator can update folder",
                ));
            }
        }

        Ok(())
    }

    pub async fn upsert_group_folder(
        &self,
        app: &tauri::AppHandle,
        core: &CoreContainer,
        folder: GroupFolderEntity,
    ) -> AppResult<GroupFolderEntity> {
        let folder = prepare_folder_for_upsert(folder);

        self.validate_group_folder_upsert(core, &folder).await?;

        self.repo.upsert_group_folder(&folder).await?;

        let event = InternalEvent::GroupFolderUpserted {
            folder_id: folder.folder_id.clone(),
            group_id: folder.group_id.clone(),
            creator_user_id: folder.creator_user_id.clone(),
            time: folder.updated_at,
        };
        emit_to_group_members(core, &self.repo, &folder.group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &folder.group_id, &event).await?;

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

    pub async fn delete_group_folder(
        &self,
        app: &tauri::AppHandle,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        folder_id: String,
    ) -> AppResult<()> {
        core.require_user_context(&user_id)?;
        let operator = self.ensure_group_member(&group_id, &user_id).await?;

        let folder = self
            .repo
            .get_group_folder_by_id(&folder_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("folder {} not found", folder_id)))?;

        if folder.group_id != group_id {
            return Err(AppError::validation(
                "folder does not belong to the target group",
            ));
        }

        // Only the creator or owner/admin can delete a folder.
        if folder.creator_user_id != user_id && matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin or creator can delete folder",
            ));
        }

        let deleted = self.repo.delete_group_folder(&folder_id).await?;
        if !deleted {
            return Err(AppError::validation("folder not found or contains files"));
        }

        let event = InternalEvent::GroupFolderDeleted {
            folder_id: folder_id.clone(),
            group_id: group_id.clone(),
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

        Ok(())
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
        app: &tauri::AppHandle,
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

        let event = InternalEvent::GroupEssenceUpdated {
            essence_id: essence.essence_id.clone(),
            group_id: essence.group_id.clone(),
            message_id: essence.message_id.clone(),
            sender_user_id: essence.sender_user_id.clone(),
            operator_user_id: essence.operator_user_id.clone(),
            is_set: essence.is_set,
            time: essence.created_at,
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

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

        // Validate path is within allowed directory, then return the relative path
        // so the frontend can use convertFileSrc() to build an asset:// URL.
        storage::validate_group_file_path(&file_path, &app_data_dir).await?;

        Ok(file_path)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upload_group_file(
        &self,
        app: &tauri::AppHandle,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        parent_folder_id: Option<String>,
        file_name: String,
        source_path: String,
        app_data_dir: PathBuf,
    ) -> AppResult<GroupFileEntity> {
        core.require_user_context(&user_id)?;
        self.ensure_group_member(&group_id, &user_id).await?;

        if let Some(parent_folder_id) = parent_folder_id.as_deref() {
            let folder = self
                .repo
                .get_group_folder_by_id(parent_folder_id)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("folder {} not found", parent_folder_id))
                })?;
            ensure_parent_folder_belongs_to_group(&folder, &group_id)?;
        }

        let file_id = crate::utils::new_db_id();
        let src = std::path::Path::new(&source_path);

        let file_path =
            storage::copy_file_to_groups_dir(src, &group_id, &file_id, &file_name, &app_data_dir)
                .await?;

        let metadata = tokio::fs::metadata(&app_data_dir.join(&file_path))
            .await
            .map_err(|e| AppError::storage(format!("failed to get file metadata: {e}")))?;

        let file_hash = storage::compute_sha256(&app_data_dir.join(&file_path)).await?;

        let file = GroupFileEntity {
            file_id,
            group_id: group_id.clone(),
            parent_folder_id,
            file_name,
            file_size: metadata.len(),
            file_hash: Some(file_hash),
            uploader_user_id: user_id,
            uploaded_at: crate::utils::now_ts(),
            expire_at: None,
            download_count: 0,
            file_path: Some(file_path),
        };

        self.repo.upsert_group_file(&file).await?;

        let event = InternalEvent::GroupFileUpserted {
            file_id: file.file_id.clone(),
            group_id: file.group_id.clone(),
            uploader_user_id: file.uploader_user_id.clone(),
            time: file.uploaded_at,
        };
        emit_to_group_members(core, &self.repo, &file.group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &file.group_id, &event).await?;

        Ok(file)
    }

    pub async fn delete_group_file(
        &self,
        app: &tauri::AppHandle,
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

        // Delete from disk first to avoid orphan files if disk delete fails.
        if let Some(ref file_path) = file.file_path {
            storage::delete_group_file_disk(file_path, &app_data_dir).await?;
        }

        self.repo.delete_group_file(&file_id).await?;

        let event = InternalEvent::GroupFileDeleted {
            file_id: file_id.clone(),
            group_id: group_id.clone(),
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

        Ok(())
    }

    pub async fn create_group_album(
        &self,
        app: &tauri::AppHandle,
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

        let event = InternalEvent::GroupAlbumCreated {
            album_id: album.album_id.clone(),
            group_id: album.group_id.clone(),
            name: album.name.clone(),
            time: album.created_at,
        };
        emit_to_group_members(core, &self.repo, &album.group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &album.group_id, &event).await?;

        Ok(album)
    }

    pub async fn list_group_albums(
        &self,
        user_id: String,
        group_id: String,
        app_data_dir: &std::path::Path,
    ) -> AppResult<Vec<GroupAlbumEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        let mut albums = self.repo.list_group_albums(&group_id).await?;
        for album in &mut albums {
            album.cover_url = resolve_relative_url(album.cover_url.as_deref(), app_data_dir);
        }
        Ok(albums)
    }

    pub async fn delete_group_album(
        &self,
        app: &tauri::AppHandle,
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

        let album = self
            .repo
            .get_group_album_by_id(&album_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("album {} not found", album_id)))?;
        ensure_album_belongs_to_group(&album, &group_id)?;

        let photos = self.repo.list_group_photos(&album_id, &group_id).await?;

        // Delete database records first (in a transaction) so the source of truth
        // is updated atomically. Disk cleanup is best-effort after that.
        self.repo.delete_group_album(&album_id, &group_id).await?;

        for photo in &photos {
            if let Some(ref file_path) = photo.file_path
                && let Err(e) = storage::delete_group_file_disk(file_path, &app_data_dir).await
            {
                tracing::warn!(
                    target: "group_content",
                    album_id = %album_id,
                    group_id = %group_id,
                    file_path = %file_path,
                    error = %e,
                    "failed to delete album photo file from disk after DB cleanup"
                );
            }
        }

        let event = InternalEvent::GroupAlbumDeleted {
            album_id: album_id.clone(),
            group_id: group_id.clone(),
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upload_group_photo(
        &self,
        app: &tauri::AppHandle,
        core: &CoreContainer,
        user_id: String,
        group_id: String,
        album_id: String,
        source_path: String,
        description: Option<String>,
        app_data_dir: PathBuf,
    ) -> AppResult<GroupPhotoEntity> {
        core.require_user_context(&user_id)?;
        self.ensure_group_member(&group_id, &user_id).await?;

        // Verify album belongs to the group
        let album = self
            .repo
            .get_group_album_by_id(&album_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("album {} not found", album_id)))?;

        if album.group_id != group_id {
            return Err(AppError::validation(
                "album does not belong to the target group",
            ));
        }

        let photo_id = crate::utils::new_db_id();

        let src = std::path::Path::new(&source_path);
        let file_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&photo_id)
            .to_string();

        let file_path =
            storage::copy_file_to_groups_dir(src, &group_id, &photo_id, &file_name, &app_data_dir)
                .await?;

        let absolute_path = app_data_dir.join(&file_path);
        let metadata = tokio::fs::metadata(&absolute_path)
            .await
            .map_err(|e| AppError::storage(format!("failed to get photo metadata: {e}")))?;

        // Store the relative path in the database so records stay portable when
        // the app data directory moves. Absolute paths are resolved only at the
        // read boundary (see `resolve_photo_url` / `resolve_album_cover_url`).
        let mut photo = GroupPhotoEntity {
            photo_id,
            album_id: album_id.clone(),
            group_id: group_id.clone(),
            url: file_path.clone(),
            file_path: Some(file_path),
            description,
            uploader_user_id: user_id.clone(),
            file_size: Some(metadata.len()),
            created_at: now_ts(),
        };

        self.repo.create_group_photo(&photo).await?;

        // Set album cover if this is the first uploaded photo. The conditional
        // UPDATE (cover_url IS NULL) makes concurrent first-uploads safe: only
        // the first writer wins, instead of last-write-wins on a stale snapshot.
        self.repo
            .set_album_cover_if_unset(&album_id, &photo.url)
            .await?;

        let event = InternalEvent::GroupPhotoUploaded {
            photo_id: photo.photo_id.clone(),
            album_id: photo.album_id.clone(),
            group_id: photo.group_id.clone(),
            time: photo.created_at,
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

        // Return an absolute URL to the caller for immediate rendering.
        photo.url = absolute_path.to_string_lossy().to_string();
        Ok(photo)
    }

    pub async fn list_group_photos(
        &self,
        user_id: String,
        group_id: String,
        album_id: String,
        app_data_dir: &std::path::Path,
    ) -> AppResult<Vec<GroupPhotoEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;
        let mut photos = self.repo.list_group_photos(&album_id, &group_id).await?;
        for photo in &mut photos {
            if let Some(resolved) = resolve_relative_url(Some(&photo.url), app_data_dir) {
                photo.url = resolved;
            }
        }
        Ok(photos)
    }

    pub async fn delete_group_photo(
        &self,
        app: &tauri::AppHandle,
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

        // Delete from disk first to avoid orphan files if disk delete fails.
        if let Some(ref file_path) = photo.file_path {
            storage::delete_group_file_disk(file_path, &app_data_dir).await?;
        }

        self.repo
            .delete_group_photo_and_refresh_cover(&photo_id, &group_id)
            .await?;

        let event = InternalEvent::GroupPhotoDeleted {
            photo_id: photo_id.clone(),
            album_id: photo.album_id.clone(),
            group_id: group_id.clone(),
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, &group_id, event.clone()).await?;
        emit_group_content_to_windows(app, &self.repo, &group_id, &event).await?;

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
        limit: i64,
    ) -> AppResult<Vec<GroupEventEntity>> {
        self.ensure_group_member(&group_id, &user_id).await?;

        let rows = self.repo.list_group_events(&group_id, limit).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn announcement(id: &str) -> GroupAnnouncementEntity {
        GroupAnnouncementEntity {
            announcement_id: id.to_string(),
            group_id: "20001".to_string(),
            sender_user_id: "10001".to_string(),
            content: "hello".to_string(),
            image_url: None,
            created_at: 100,
            updated_at: 100,
        }
    }

    fn folder(id: &str) -> GroupFolderEntity {
        GroupFolderEntity {
            folder_id: id.to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: None,
            folder_name: "docs".to_string(),
            creator_user_id: "10001".to_string(),
            created_at: 100,
            updated_at: 100,
            file_count: 0,
        }
    }

    #[test]
    fn prepares_new_announcement_with_generated_id() {
        let prepared = prepare_announcement_for_upsert(announcement(""));

        assert!(!prepared.announcement_id.is_empty());
        assert_eq!(prepared.group_id, "20001");
        assert_eq!(prepared.sender_user_id, "10001");
    }

    #[test]
    fn preserves_existing_announcement_id() {
        let prepared = prepare_announcement_for_upsert(announcement("ann-1"));

        assert_eq!(prepared.announcement_id, "ann-1");
    }

    #[test]
    fn prepares_new_folder_with_generated_id() {
        let prepared = prepare_folder_for_upsert(folder(""));

        assert!(!prepared.folder_id.is_empty());
        assert_eq!(prepared.group_id, "20001");
        assert_eq!(prepared.creator_user_id, "10001");
    }

    #[test]
    fn preserves_existing_folder_id() {
        let prepared = prepare_folder_for_upsert(folder("folder-1"));

        assert_eq!(prepared.folder_id, "folder-1");
    }

    #[test]
    fn prepares_new_announcement_with_current_timestamps() {
        let before = crate::utils::now_ts();
        let prepared = prepare_announcement_for_upsert(announcement(""));
        let after = crate::utils::now_ts();

        assert!(prepared.created_at >= before && prepared.created_at <= after);
        assert!(prepared.updated_at >= before && prepared.updated_at <= after);
    }

    #[test]
    fn updates_existing_announcement_updated_at() {
        let before = crate::utils::now_ts();
        let prepared = prepare_announcement_for_upsert(announcement("ann-1"));
        let after = crate::utils::now_ts();

        assert_eq!(prepared.created_at, 100);
        assert!(prepared.updated_at >= before && prepared.updated_at <= after);
    }

    #[test]
    fn prepares_new_folder_with_current_timestamps() {
        let before = crate::utils::now_ts();
        let prepared = prepare_folder_for_upsert(folder(""));
        let after = crate::utils::now_ts();

        assert!(prepared.created_at >= before && prepared.created_at <= after);
        assert!(prepared.updated_at >= before && prepared.updated_at <= after);
    }

    #[test]
    fn updates_existing_folder_updated_at() {
        let before = crate::utils::now_ts();
        let prepared = prepare_folder_for_upsert(folder("folder-1"));
        let after = crate::utils::now_ts();

        assert_eq!(prepared.created_at, 100);
        assert!(prepared.updated_at >= before && prepared.updated_at <= after);
    }

    #[test]
    fn accepts_album_in_target_group() {
        let album = GroupAlbumEntity {
            album_id: "album-1".to_string(),
            group_id: "20001".to_string(),
            name: "album".to_string(),
            cover_url: None,
            photo_count: 0,
            created_at: 100,
            updated_at: 100,
        };

        assert!(ensure_album_belongs_to_group(&album, "20001").is_ok());
    }

    #[test]
    fn rejects_album_from_other_group() {
        let album = GroupAlbumEntity {
            album_id: "album-1".to_string(),
            group_id: "20002".to_string(),
            name: "album".to_string(),
            cover_url: None,
            photo_count: 0,
            created_at: 100,
            updated_at: 100,
        };

        assert!(ensure_album_belongs_to_group(&album, "20001").is_err());
    }

    #[test]
    fn accepts_parent_folder_in_target_group() {
        let folder = folder("folder-1");

        assert!(ensure_parent_folder_belongs_to_group(&folder, "20001").is_ok());
    }

    #[test]
    fn rejects_parent_folder_from_other_group() {
        let mut folder = folder("folder-1");
        folder.group_id = "20002".to_string();

        assert!(ensure_parent_folder_belongs_to_group(&folder, "20001").is_err());
    }

    #[test]
    fn resolve_relative_url_joins_relative_path() {
        let base = std::path::Path::new("/data/app");
        let resolved = resolve_relative_url(Some("groups/20001/files/a.png"), base);
        assert_eq!(
            resolved.as_deref(),
            Some("/data/app/groups/20001/files/a.png")
        );
    }

    #[test]
    fn resolve_relative_url_passes_through_absolute_path() {
        let base = std::path::Path::new("/data/app");
        let resolved = resolve_relative_url(Some("/legacy/abs/a.png"), base);
        assert_eq!(resolved.as_deref(), Some("/legacy/abs/a.png"));
    }

    #[test]
    fn resolve_relative_url_handles_none_and_empty() {
        let base = std::path::Path::new("/data/app");
        assert_eq!(resolve_relative_url(None, base), None);
        assert_eq!(resolve_relative_url(Some(""), base), None);
    }
}
