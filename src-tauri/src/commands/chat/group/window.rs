use tauri::Manager;

use crate::error::{AppError, AppResult};
use crate::services::ServiceHub;

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
}
