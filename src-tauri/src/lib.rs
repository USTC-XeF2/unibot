mod commands;
pub mod core;
pub mod error;
pub mod logging;
pub mod models;
pub mod persistence;
pub mod protocol;
pub mod services;
pub mod utils;

use tauri::Manager;

use commands::{
    bot,
    chat::{conversation, group, message, request, user},
    log, main, packet,
};
use core::CoreContainer;
use persistence::{
    BotRepo, ConversationRepo, GroupRepo, InteractionRepo, MessageRepo, SettingsRepo, UserRepo,
    init_sqlite_pool,
};
use protocol::ProtocolRuntimeManager;
use services::{
    BotService, ConversationService, GroupService, InteractionService, MessageService,
    RequestService, ServiceHub, SettingsService, UserService,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(init_sqlite_pool(&app.handle().clone()))?;
            let user_repo = UserRepo::new(pool.clone());
            let message_repo = MessageRepo::new(pool.clone());
            let interaction_repo = InteractionRepo::new(pool.clone());
            let group_repo = GroupRepo::new(pool.clone());
            let bot_repo = BotRepo::new(pool.clone());
            let request_user_repo = user_repo.clone();
            let message_repo_for_interaction = message_repo.clone();
            let service_hub = ServiceHub::new(
                MessageService::new(message_repo.clone(), group_repo.clone()),
                InteractionService::new(
                    interaction_repo,
                    message_repo_for_interaction,
                    group_repo.clone(),
                ),
                GroupService::new(group_repo.clone(), message_repo.clone()),
                RequestService::new(request_user_repo),
                UserService::new(user_repo.clone()),
                BotService::new(bot_repo.clone()),
                SettingsService::new(SettingsRepo::new(pool.clone())),
                ConversationService::new(ConversationRepo::new(pool.clone())),
            );

            let core = CoreContainer::new();
            let persisted_users = tauri::async_runtime::block_on(service_hub.user.list_users())
                .map_err(|err| format!("failed to load users from db: {err}"))?;
            for profile in persisted_users {
                if core.user_context(&profile.user_id).is_none() {
                    core.register_user(profile)
                        .map_err(|err| format!("failed to restore user to memory: {err}"))?;
                }
            }

            // Reset any bots that were left in "running" state from a previous
            // unclean shutdown (e.g. app crashed or was force-killed).
            tauri::async_runtime::block_on(bot_repo.reset_all_running_bots())
                .map_err(|err| format!("failed to reset running bots: {err}"))?;

            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|err| format!("failed to get app data dir: {err}"))?;

            // Ensure groups directory exists for file storage
            let groups_dir = app_data_dir.join("groups");
            std::fs::create_dir_all(&groups_dir)
                .map_err(|err| format!("failed to create groups dir: {err}"))?;

            // Initialize tracing/logging
            let log_level = tauri::async_runtime::block_on(service_hub.settings.get_log_level());
            let logs_dir = app_data_dir.join("logs");
            std::fs::create_dir_all(&logs_dir)
                .map_err(|err| format!("failed to create logs dir: {err}"))?;
            let log_guard = logging::init_logging(&logs_dir, &log_level)
                .map_err(|err| format!("failed to init logging: {err}"))?;
            tracing::info!(target: "app", "UniBot started, log_level={}", log_level);

            // Schedule periodic log cleanup.
            //
            // Strategy: read `log.last_cleanup_at` on startup.
            // If the last cleanup happened before today's 4:00 AM,
            // run one immediately (covers missed cleanups).
            // Then wait until next 4:00 AM and repeat.
            {
                let cleanup_app = app.handle().clone();
                let cleanup_hub = service_hub.clone();
                tauri::async_runtime::spawn(async move {
                    async fn run_cleanup(
                        app: &tauri::AppHandle,
                        hub: &crate::services::ServiceHub,
                    ) -> bool {
                        let log_dir = match app.path().app_data_dir() {
                            Ok(dir) => dir.join("logs"),
                            Err(e) => {
                                tracing::warn!(target: "log_cleanup", "failed to get app data dir: {e}");
                                return false;
                            }
                        };
                        let retention_days = hub.settings.get_log_retention_days().await;
                        if retention_days <= 0 {
                            return false;
                        }
                        match crate::logging::cleanup_old_logs(&log_dir, retention_days).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!(
                                        target: "log_cleanup",
                                        "auto-cleaned {deleted} old log files (retention={retention_days}d)"
                                    );
                                }
                                true
                            }
                            Err(e) => {
                                tracing::warn!(target: "log_cleanup", "auto-cleanup failed: {e}");
                                false
                            }
                        }
                    }

                    // --- Startup check ---
                    let last_cleanup = cleanup_hub.settings.get_log_last_cleanup_at().await;
                    let today_4am = chrono::Local::now()
                        .date_naive()
                        .and_hms_opt(4, 0, 0)
                        .expect("4:00 is a valid time")
                        .and_utc()
                        .timestamp_millis();

                    // If never cleaned, or last cleanup was before today's 4:00 AM
                    if last_cleanup == 0 || last_cleanup < today_4am {
                        if run_cleanup(&cleanup_app, &cleanup_hub).await {
                            let _ = cleanup_hub
                                .settings
                                .set_log_last_cleanup_at(crate::utils::now_ts() as i64)
                                .await;
                        }
                    }

                    let mut next_run = tokio::time::Instant::now();
                    loop {
                        // Wait until next 4:00 AM
                        let now = chrono::Local::now();
                        let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
                        let tomorrow_4am = match tomorrow
                            .and_hms_opt(4, 0, 0)
                            .expect("4:00 is a valid time")
                            .and_local_timezone(chrono::Local)
                        {
                            chrono::LocalResult::Single(dt) => dt,
                            chrono::LocalResult::Ambiguous(dt, _) => dt,
                            chrono::LocalResult::None => now + chrono::TimeDelta::hours(24),
                        };
                        let delay_ms =
                            (tomorrow_4am - now).num_milliseconds().max(0) as u64;

                        tokio::time::sleep_until(
                            next_run + std::time::Duration::from_millis(delay_ms),
                        )
                        .await;
                        next_run = tokio::time::Instant::now();

                        if run_cleanup(&cleanup_app, &cleanup_hub).await {
                            let _ = cleanup_hub
                                .settings
                                .set_log_last_cleanup_at(crate::utils::now_ts() as i64)
                                .await;
                        }
                    }
                });
            }

            let protocol_runtime = protocol::ProtocolRuntimeManager::new(
                bot_repo.clone(),
                service_hub.clone(),
                core.clone(),
                app_data_dir,
                pool.clone(),
            );

            app.manage(pool.clone());
            app.manage(core);
            app.manage(service_hub);
            app.manage(protocol_runtime);
            app.manage(log_guard);

            if let Some(main_window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    if matches!(
                        event,
                        tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
                    ) {
                        // Gracefully shut down all protocol servers before exit.
                        // Spawn a dedicated thread and block_on so shutdown actually
                        // completes before the process exits. Cap wait at 3s so UI
                        // doesn't freeze if servers hang.
                        let app_handle_shutdown = app_handle.clone();
                        let handle = std::thread::spawn(move || {
                            tauri::async_runtime::block_on(async move {
                                let runtime = app_handle_shutdown.state::<ProtocolRuntimeManager>();
                                runtime.shutdown_all().await;
                            });
                        });
                        let start = std::time::Instant::now();
                        while start.elapsed() < std::time::Duration::from_secs(3) {
                            if handle.is_finished() {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                        if !handle.is_finished() {
                            tracing::warn!(target: "app", "shutdown thread timed out after 3s, exiting anyway");
                        }

                        let core_state = app_handle.state::<CoreContainer>();
                        for context in core_state.list_user_contexts() {
                            if let Some(label) = context.chat_window_label() {
                                if let Some(window) = app_handle.get_webview_window(&label) {
                                    let _ = window.close();
                                }
                                context.clear_chat_window_label();
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_prevent_default::debug())
        .invoke_handler(tauri::generate_handler![
            main::register_user,
            main::list_users,
            main::list_groups,
            main::delete_user,
            main::open_user_chat_window,
            main::get_db_status,
            main::get_stats,
            log::list_system_logs,
            log::get_log_settings,
            log::set_log_level,
            log::set_log_retention_days,
            log::trigger_log_cleanup,
            bot::create_bot,
            bot::get_bot_config,
            bot::list_bots,
            bot::rename_bot,
            bot::delete_bot,
            bot::start_bot,
            bot::stop_bot,
            bot::list_debug_sessions,
            user::update_user_profile,
            user::list_friends,
            user::list_user_groups,
            message::send_message,
            message::list_message_history,
            message::recall_message,
            message::react_to_message,
            message::poke_user,
            message::list_poke_history,
            request::create_friend_request,
            request::list_friend_requests,
            request::handle_friend_request,
            request::delete_friend,
            group::upsert_group,
            group::upsert_group_member,
            group::list_group_members,
            group::list_group_event_history,
            group::mute_group_member,
            group::set_group_whole_mute,
            group::get_group_whole_mute,
            group::create_group_request,
            group::list_group_requests,
            group::handle_group_request,
            group::kick_group_member,
            group::set_group_member_role,
            group::set_group_member_title,
            group::rename_group,
            group::leave_group,
            group::dissolve_group,
            group::upsert_group_announcement,
            group::list_group_announcements,
            group::upsert_group_folder,
            group::list_group_folders,
            group::upsert_group_file,
            group::list_group_files,
            group::upload_group_file,
            group::download_group_file,
            group::delete_group_file,
            group::create_group_album,
            group::list_group_albums,
            group::delete_group_album,
            group::upload_group_photo,
            group::list_group_photos,
            group::delete_group_photo,
            group::set_group_essence_message,
            group::list_group_essence_messages,
            conversation::set_conversation_pinned,
            conversation::set_conversation_muted,
            conversation::list_conversation_states,
            conversation::list_group_categories,
            conversation::create_group_category,
            conversation::delete_group_category,
            conversation::set_group_category,
            packet::list_protocol_packets,
            packet::read_protocol_packet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
