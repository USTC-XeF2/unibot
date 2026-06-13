use tauri::Manager;

use crate::core::CoreContainer;
use crate::error::AppResult;
use crate::services::ServiceHub;

use super::super::super::IntoCommandResult;

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

async fn resolve_group_name(
    services: &ServiceHub,
    core: &CoreContainer,
    user_id: &str,
    group_id: &str,
) -> AppResult<String> {
    let groups = services
        .group
        .list_user_groups(core, user_id.to_string())
        .await?;
    Ok(groups
        .into_iter()
        .find(|g| g.group_id == group_id)
        .map(|g| g.group_name)
        .unwrap_or_else(|| group_id.to_string()))
}

async fn open_group_files_window_impl(
    app: tauri::AppHandle,
    core: &CoreContainer,
    services: &ServiceHub,
    user_id: String,
    group_id: String,
) -> AppResult<bool> {
    let label = group_content_window_label("group-files", &user_id, &group_id);
    if !ensure_or_focus_window(app.clone(), &label)? {
        return Ok(false);
    }

    let group_name = resolve_group_name(services, core, &user_id, &group_id).await?;
    let title = format!("群文件 · {}", group_name);

    let url = tauri::WebviewUrl::App(
        format!("index.html#/group-files?userId={user_id}&groupId={group_id}").into(),
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
    core: &CoreContainer,
    services: &ServiceHub,
    user_id: String,
    group_id: String,
) -> AppResult<bool> {
    let label = group_content_window_label("group-albums", &user_id, &group_id);
    if !ensure_or_focus_window(app.clone(), &label)? {
        return Ok(false);
    }

    let group_name = resolve_group_name(services, core, &user_id, &group_id).await?;
    let title = format!("群相册 · {}", group_name);

    let url = tauri::WebviewUrl::App(
        format!("index.html#/group-albums?userId={user_id}&groupId={group_id}").into(),
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
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<bool, String> {
    open_group_files_window_impl(app, &core, &services, user_id, group_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn open_group_albums_window(
    app: tauri::AppHandle,
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    group_id: String,
) -> Result<bool, String> {
    open_group_albums_window_impl(app, &core, &services, user_id, group_id)
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
}
