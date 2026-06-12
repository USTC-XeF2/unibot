use tauri::{Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};

#[tauri::command]
pub fn open_developer_tools(
    app: tauri::AppHandle,
    core: tauri::State<CoreContainer>,
) -> AppResult<bool> {
    let label = "developer-tools";

    if let Some(existing) = app.get_webview_window(label) {
        existing.show().map_err(|e| {
            AppError::internal(format!("failed to show developer tools window: {e}"))
        })?;
        existing.unminimize().map_err(|e| {
            AppError::internal(format!("failed to unminimize developer tools window: {e}"))
        })?;
        existing.set_focus().map_err(|e| {
            AppError::internal(format!("failed to focus developer tools window: {e}"))
        })?;
        return Ok(false);
    }

    let webview_url = tauri::WebviewUrl::App(format!("index.html#/developer-tools").into());
    let _ = tauri::WebviewWindowBuilder::new(&app, label, webview_url)
        .title("开发者工具")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .build()
        .map_err(|e| AppError::internal(format!("failed to create developer tools window: {e}")))?;

    let mut devtools_rx = core.subscribe_devtools_events();
    let app_handle = app.clone();
    let label_owned = label.to_string();

    tauri::async_runtime::spawn(async move {
        loop {
            match devtools_rx.recv().await {
                Ok(devtools_event) => {
                    if app_handle.get_webview_window(&label_owned).is_none() {
                        break;
                    }
                    if let Err(e) =
                        app_handle.emit_to(&label_owned, "devtools:event", &devtools_event)
                    {
                        tracing::error!(target: "dev_tools", "emit_to developer-tools failed: {}", e);
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    Ok(true)
}
