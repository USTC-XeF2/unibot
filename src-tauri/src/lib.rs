mod commands;
mod core;
mod error;
mod models;
mod persistence;
mod services;
mod utils;

use tauri::Manager;

use commands::{
    chat::{group, message, request, user},
    main,
};
use core::CoreContainer;
use persistence::{GroupRepo, InteractionRepo, MessageRepo, UserRepo, init_sqlite_pool};
use services::{
    GroupService, InteractionService, MessageService, RequestService, ServiceHub, UserService,
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
            );

            let core = CoreContainer::new();
            let persisted_users = tauri::async_runtime::block_on(service_hub.user.list_users())
                .map_err(|err| format!("failed to load users from db: {err}"))?;
            for profile in persisted_users {
                if core.user_context(profile.user_id).is_none() {
                    core.register_user(profile)
                        .map_err(|err| format!("failed to restore user to memory: {err}"))?;
                }
            }

            app.manage(core);
            app.manage(service_hub);

            if let Some(main_window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    if matches!(
                        event,
                        tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
                    ) {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
