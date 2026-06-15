use tauri::Manager;

use crate::error::{AppError, AppResult};
use crate::services::{GroupService, ServiceHub};

use super::super::super::IntoCommandResult;

/// Allowed characters for user/group ID components used in window labels and
/// URLs. Restricting to this set prevents label collisions, illegal window
/// labels, and query-parameter injection.
const ID_ALLOWED_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";

fn validate_id_component(id: &str) -> AppResult<()> {
    if id.is_empty() {
        return Err(AppError::validation("id must not be empty"));
    }
    if id.len() > 128 {
        return Err(AppError::validation("id must not exceed 128 characters"));
    }
    if id.chars().all(|c| ID_ALLOWED_CHARS.contains(c)) {
        Ok(())
    } else {
        Err(AppError::validation(
            "id contains invalid characters (allowed: A-Z, a-z, 0-9, _, -)",
        ))
    }
}

fn group_content_window_label(kind: &str, user_id: &str, group_id: &str) -> String {
    format!("{kind}-{user_id}-{group_id}")
}

pub(crate) fn group_content_window_labels(user_id: &str, group_id: &str) -> [String; 2] {
    [
        group_content_window_label("group-files", user_id, group_id),
        group_content_window_label("group-albums", user_id, group_id),
    ]
}

pub(crate) fn close_group_content_windows(app: &tauri::AppHandle, user_id: &str, group_id: &str) {
    for label in group_content_window_labels(user_id, group_id) {
        if let Some(window) = app.get_webview_window(&label)
            && let Err(error) = window.close()
        {
            tracing::warn!(
                target: "group_content",
                window_label = %label,
                %error,
                "failed to close revoked group content window"
            );
        }
    }
}

async fn ensure_group_content_window_access(
    group_service: &GroupService,
    user_id: &str,
    group_id: &str,
) -> AppResult<()> {
    group_service
        .ensure_group_member(group_id, user_id)
        .await
        .map(|_| ())
}

fn ensure_or_focus_window(app: tauri::AppHandle, label: &str) -> AppResult<bool> {
    if let Some(existing) = app.get_webview_window(label) {
        existing
            .show()
            .map_err(|e| crate::error::AppError::internal(format!("failed to show window: {e}")))?;
        existing.unminimize().map_err(|e| {
            crate::error::AppError::internal(format!("failed to unminimize window: {e}"))
        })?;
        existing.set_focus().map_err(|e| {
            crate::error::AppError::internal(format!("failed to focus window: {e}"))
        })?;
        return Ok(false);
    }
    Ok(true)
}

async fn resolve_group_name(services: &ServiceHub, group_id: &str) -> AppResult<String> {
    services.group.get_group(group_id).await.map(|group| {
        group
            .map(|g| g.group_name)
            .unwrap_or_else(|| group_id.to_string())
    })
}

async fn open_group_files_window_impl(
    app: tauri::AppHandle,
    services: &ServiceHub,
    user_id: String,
    group_id: String,
) -> AppResult<bool> {
    validate_id_component(&user_id)?;
    validate_id_component(&group_id)?;
    ensure_group_content_window_access(&services.group, &user_id, &group_id).await?;

    let label = group_content_window_label("group-files", &user_id, &group_id);
    if !ensure_or_focus_window(app.clone(), &label)? {
        return Ok(false);
    }

    let group_name = resolve_group_name(services, &group_id).await?;
    let title = format!("群文件 · {}", group_name);

    let url = tauri::WebviewUrl::App(
        format!(
            "index.html#/group-files?userId={}&groupId={}",
            urlencoding::encode(&user_id),
            urlencoding::encode(&group_id)
        )
        .into(),
    );
    tauri::WebviewWindowBuilder::new(&app, label, url)
        .title(title)
        .inner_size(960.0, 680.0)
        .min_inner_size(520.0, 420.0)
        .center()
        .build()
        .map_err(|e| crate::error::AppError::internal(format!("failed to create window: {e}")))?;

    Ok(true)
}

async fn open_group_albums_window_impl(
    app: tauri::AppHandle,
    services: &ServiceHub,
    user_id: String,
    group_id: String,
) -> AppResult<bool> {
    validate_id_component(&user_id)?;
    validate_id_component(&group_id)?;
    ensure_group_content_window_access(&services.group, &user_id, &group_id).await?;

    let label = group_content_window_label("group-albums", &user_id, &group_id);
    if !ensure_or_focus_window(app.clone(), &label)? {
        return Ok(false);
    }

    let group_name = resolve_group_name(services, &group_id).await?;
    let title = format!("群相册 · {}", group_name);

    let url = tauri::WebviewUrl::App(
        format!(
            "index.html#/group-albums?userId={}&groupId={}",
            urlencoding::encode(&user_id),
            urlencoding::encode(&group_id)
        )
        .into(),
    );
    tauri::WebviewWindowBuilder::new(&app, label, url)
        .title(title)
        .inner_size(960.0, 680.0)
        .min_inner_size(520.0, 420.0)
        .center()
        .build()
        .map_err(|e| crate::error::AppError::internal(format!("failed to create window: {e}")))?;

    Ok(true)
}

#[tauri::command]
pub async fn open_group_files_window(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<bool, String> {
    open_group_files_window_impl(app, &services, user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn open_group_albums_window(
    app: tauri::AppHandle,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<bool, String> {
    open_group_albums_window_impl(app, &services, user_id, group_id)
        .await
        .into_command_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GroupMemberProfile, GroupProfile, GroupRole, UserProfile};
    use crate::persistence::{GroupRepo, MessageRepo, UserRepo, migrator};
    use crate::services::GroupService;

    #[test]
    fn group_content_window_label_format() {
        assert_eq!(
            group_content_window_label("group-files", "u1", "g1"),
            "group-files-u1-g1"
        );
        assert_eq!(
            group_content_window_label("group-albums", "u1", "g1"),
            "group-albums-u1-g1"
        );
    }

    #[test]
    fn group_content_window_labels_include_files_and_albums() {
        assert_eq!(
            group_content_window_labels("u1", "g1"),
            [
                "group-files-u1-g1".to_string(),
                "group-albums-u1-g1".to_string(),
            ]
        );
    }

    #[test]
    fn validate_id_component_accepts_alphanumeric_dash_underscore() {
        assert!(validate_id_component("user_123-ABC").is_ok());
    }

    #[test]
    fn validate_id_component_rejects_empty() {
        assert!(validate_id_component("").is_err());
    }

    #[test]
    fn validate_id_component_rejects_special_characters() {
        for invalid in ["u/1", "u&1", "u 1", "u?1", "u#1", "u.1", "u\\1"] {
            assert!(
                validate_id_component(invalid).is_err(),
                "expected {invalid:?} to be rejected"
            );
        }
    }

    #[test]
    fn validate_id_component_rejects_too_long() {
        let long_id = "a".repeat(129);
        assert!(validate_id_component(&long_id).is_err());
    }

    #[sqlx::test]
    async fn group_content_window_access_rejects_non_member(pool: sqlx::SqlitePool) {
        migrator::run_migrations(&pool).await.unwrap();

        let user_repo = UserRepo::new(pool.clone());
        user_repo
            .upsert_user(&UserProfile {
                user_id: "10001".to_string(),
                nickname: "Owner".to_string(),
                avatar: String::new(),
                signature: String::new(),
                account_status: Default::default(),
            })
            .await
            .unwrap();
        user_repo
            .upsert_user(&UserProfile {
                user_id: "10002".to_string(),
                nickname: "Removed".to_string(),
                avatar: String::new(),
                signature: String::new(),
                account_status: Default::default(),
            })
            .await
            .unwrap();

        let group_repo = GroupRepo::new(pool.clone());
        group_repo
            .upsert_group(&GroupProfile {
                group_id: "20001".to_string(),
                group_name: "Test".to_string(),
                owner_user_id: "10001".to_string(),
                member_count: 1,
                max_member_count: 500,
                group_status: Default::default(),
                category_id: None,
            })
            .await
            .unwrap();
        group_repo
            .upsert_group_member(&GroupMemberProfile {
                group_id: "20001".to_string(),
                user_id: "10001".to_string(),
                card: String::new(),
                title: String::new(),
                role: GroupRole::Owner,
                joined_at: 1,
                last_sent_at: 0,
                mute_until: None,
            })
            .await
            .unwrap();

        let service = GroupService::new(group_repo, MessageRepo::new(pool));
        assert!(
            ensure_group_content_window_access(&service, "10002", "20001")
                .await
                .is_err()
        );
    }
}
