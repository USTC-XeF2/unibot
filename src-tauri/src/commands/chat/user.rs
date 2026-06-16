use crate::core::CoreContainer;
use crate::models::{DbId, FriendCategoryEntity, FriendshipEntity, GroupProfile, UserProfile};
use crate::services::ServiceHub;

use super::super::IntoCommandResult;

#[tauri::command]
pub async fn update_user_profile(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    nickname: Option<String>,
    avatar: Option<String>,
    signature: Option<String>,
) -> Result<UserProfile, String> {
    services
        .user
        .update_user_profile(user_id, nickname, avatar, signature)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_friends(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<DbId>, String> {
    services
        .user
        .list_friends(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_friendships(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<FriendshipEntity>, String> {
    services
        .user
        .list_friendships(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_friend_categories(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<FriendCategoryEntity>, String> {
    services
        .user
        .list_friend_categories(user_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn create_friend_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    name: String,
) -> Result<FriendCategoryEntity, String> {
    services
        .user
        .create_friend_category(user_id, name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn rename_friend_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    category_id: String,
    name: String,
) -> Result<FriendCategoryEntity, String> {
    services
        .user
        .rename_friend_category(user_id, category_id, name)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn delete_friend_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    category_id: String,
) -> Result<(), String> {
    services
        .user
        .delete_friend_category(user_id, category_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn set_friend_category(
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
    friend_user_id: String,
    category_id: String,
) -> Result<(), String> {
    services
        .user
        .set_friend_category(user_id, friend_user_id, category_id)
        .await
        .into_command_result()
}

#[tauri::command]
pub async fn list_user_groups(
    core: tauri::State<'_, CoreContainer>,
    services: tauri::State<'_, ServiceHub>,
    user_id: String,
) -> Result<Vec<GroupProfile>, String> {
    services
        .group
        .list_user_groups(&core, user_id)
        .await
        .into_command_result()
}
