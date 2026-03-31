use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tauri::{Emitter, Manager};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

use crate::error::{AppError, AppResult};
use crate::models::{InternalEvent, UserProfile};

pub const DEFAULT_EVENT_BUS_CAPACITY: usize = 256;

#[derive(Clone, Debug)]
pub struct UserContext {
    pub profile: UserProfile,
    pub event_tx: broadcast::Sender<InternalEvent>,
    chat_window_label: Arc<RwLock<Option<String>>>,
}

impl UserContext {
    pub fn new(profile: UserProfile) -> Self {
        let (event_tx, _) = broadcast::channel(DEFAULT_EVENT_BUS_CAPACITY);
        Self {
            profile,
            event_tx,
            chat_window_label: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_chat_window_label(&self, label: Option<String>) {
        *self
            .chat_window_label
            .write()
            .expect("chat window label lock poisoned") = label;
    }

    pub fn chat_window_label(&self) -> Option<String> {
        self.chat_window_label
            .read()
            .expect("chat window label lock poisoned")
            .clone()
    }

    pub fn clear_chat_window_label(&self) {
        self.set_chat_window_label(None);
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<InternalEvent> {
        self.event_tx.subscribe()
    }
}

pub struct CoreContainer {
    users: RwLock<HashMap<u64, UserContext>>,
}

impl Default for CoreContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreContainer {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_user(&self, profile: UserProfile) -> AppResult<UserContext> {
        let context = UserContext::new(profile);
        let user_id = context.profile.user_id;
        let mut users = self.users.write().expect("users lock poisoned");
        if users.contains_key(&user_id) {
            return Err(AppError::conflict(format!(
                "user {} already exists",
                user_id
            )));
        }
        users.insert(user_id, context.clone());
        Ok(context)
    }

    pub fn unregister_user(&self, user_id: u64) -> Option<UserContext> {
        self.users
            .write()
            .expect("users lock poisoned")
            .remove(&user_id)
    }

    pub fn user_context(&self, user_id: u64) -> Option<UserContext> {
        self.users
            .read()
            .expect("users lock poisoned")
            .get(&user_id)
            .cloned()
    }

    pub fn list_user_contexts(&self) -> Vec<UserContext> {
        self.users
            .read()
            .expect("users lock poisoned")
            .values()
            .cloned()
            .collect()
    }

    pub fn require_user_context(&self, user_id: u64) -> AppResult<UserContext> {
        self.user_context(user_id)
            .ok_or_else(|| AppError::not_found(format!("user {} is not registered", user_id)))
    }

    pub fn open_user_chat_window(
        &self,
        app: tauri::AppHandle,
        user_id: u64,
        nickname_hint: Option<String>,
    ) -> AppResult<bool> {
        if user_id == 0 {
            return Err(AppError::validation("invalid user id"));
        }

        let user_context = self.require_user_context(user_id)?;

        if let Some(existing_label) = user_context.chat_window_label() {
            if let Some(existing_window) = app.get_webview_window(&existing_label) {
                existing_window.show().map_err(|err| {
                    AppError::internal(format!("failed to show chat window: {err}"))
                })?;
                existing_window.unminimize().map_err(|err| {
                    AppError::internal(format!("failed to unminimize chat window: {err}"))
                })?;
                existing_window.set_focus().map_err(|err| {
                    AppError::internal(format!("failed to focus chat window: {err}"))
                })?;
                return Ok(false);
            }
            user_context.clear_chat_window_label();
        }

        let label = format!("chat-{}", user_id);
        if let Some(existing_window) = app.get_webview_window(&label) {
            existing_window
                .show()
                .map_err(|err| AppError::internal(format!("failed to show chat window: {err}")))?;
            existing_window.unminimize().map_err(|err| {
                AppError::internal(format!("failed to unminimize chat window: {err}"))
            })?;
            existing_window
                .set_focus()
                .map_err(|err| AppError::internal(format!("failed to focus chat window: {err}")))?;
            user_context.set_chat_window_label(Some(label));
            return Ok(false);
        }

        let inferred_nickname = nickname_hint
            .filter(|nickname| !nickname.trim().is_empty())
            .unwrap_or_else(|| user_context.profile.nickname.clone());

        let title = if inferred_nickname.trim().is_empty() {
            format!("聊天 · {}", user_id)
        } else {
            format!("聊天 · {}", inferred_nickname.trim())
        };

        let webview_url = tauri::WebviewUrl::App(format!("index.html#/chat/{}", user_id).into());
        let chat_window = tauri::WebviewWindowBuilder::new(&app, label.clone(), webview_url)
            .title(title)
            .inner_size(960.0, 680.0)
            .min_inner_size(520.0, 420.0)
            .center()
            .build()
            .map_err(|err| AppError::internal(format!("failed to create chat window: {err}")))?;

        let mut event_rx = user_context.subscribe_events();
        let app_handle_for_events = app.clone();
        let event_window_label = label.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        if let Some(window) =
                            app_handle_for_events.get_webview_window(&event_window_label)
                        {
                            let _ = window.emit("chat:event", &event);
                        } else {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });

        user_context.set_chat_window_label(Some(label));

        let app_handle = app.clone();
        chat_window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let core_state = app_handle.state::<CoreContainer>();
                if let Some(context) = core_state.user_context(user_id) {
                    context.clear_chat_window_label();
                }
            }
        });

        Ok(true)
    }
}
