mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod persistence;
pub mod protocol;
pub mod services;
pub mod utils;

use tauri::Manager;

use commands::{
    bot,
    chat::{group, message, request, user},
    main, packet,
};
use core::CoreContainer;
use persistence::{BotRepo, GroupRepo, InteractionRepo, MessageRepo, UserRepo, init_sqlite_pool};
use protocol::ProtocolRuntimeManager;
use services::{
    BotService, GroupService, InteractionService, MessageService, RequestService, ServiceHub,
    UserService,
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
                GroupService::new(group_repo, message_repo.clone()),
                RequestService::new(request_user_repo),
                UserService::new(user_repo.clone()),
                BotService::new(bot_repo.clone()),
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

            if let Some(main_window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    if matches!(
                        event,
                        tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
                    ) {
                        // Gracefully shut down all protocol servers before exit.
                        // Use spawn instead of block_on to avoid blocking the main thread.
                        let app_handle_shutdown = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let runtime = app_handle_shutdown.state::<ProtocolRuntimeManager>();
                            runtime.shutdown_all().await;
                        });

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
        .plugin(tauri_plugin_prevent_default::debug())
        .invoke_handler(tauri::generate_handler![
            main::register_user,
            main::list_users,
            main::list_groups,
            main::delete_user,
            main::open_user_chat_window,
            main::get_db_status,
            main::get_stats,
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
            group::set_group_essence_message,
            group::list_group_essence_messages,
            packet::list_protocol_packets,
            packet::read_protocol_packet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
