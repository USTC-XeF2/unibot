use crate::core::CoreContainer;
use crate::models::{
    GroupAlbumEntity, GroupAnnouncementEntity, GroupFileEntity, GroupFolderEntity,
    GroupMemberProfile, GroupPhotoEntity, GroupProfile, GroupRequestType, GroupRole, RequestState,
};
use crate::persistence::{
    BotRepo, GroupRepo, InteractionRepo, MessageRepo, NewFriendRequestRecord, NewGroupEventRecord,
    NewGroupRequestRecord, NewMessageReactionRecord, NewMessageRecord, NewPokeRecord, UserRepo,
    migrator,
};
use crate::services::GroupService;

async fn setup(pool: &sqlx::SqlitePool) {
    migrator::run_migrations(pool)
        .await
        .expect("migrations should succeed");
}

fn make_profile(user_id: &str, nickname: &str) -> crate::models::UserProfile {
    use crate::models::UserProfile;
    UserProfile {
        user_id: user_id.to_string(),
        nickname: nickname.to_string(),
        avatar: "".to_string(),
        signature: "".to_string(),
        account_status: Default::default(),
    }
}

fn make_group(group_id: &str, name: &str, owner_id: &str) -> GroupProfile {
    GroupProfile {
        group_id: group_id.to_string(),
        group_name: name.to_string(),
        owner_user_id: owner_id.to_string(),
        member_count: 0,
        max_member_count: 500,
        group_status: Default::default(),
        category_id: None,
    }
}

fn make_member(group_id: &str, user_id: &str, role: GroupRole) -> GroupMemberProfile {
    GroupMemberProfile {
        group_id: group_id.to_string(),
        user_id: user_id.to_string(),
        card: "".to_string(),
        title: "".to_string(),
        role,
        joined_at: 1,
        last_sent_at: 0,
        mute_until: None,
    }
}

fn register_test_user(core: &CoreContainer, user_id: &str, nickname: &str) {
    core.register_user(make_profile(user_id, nickname))
        .expect("test user should register");
}

mod account;
mod bots;
mod categories;
mod group_albums;
mod group_announcements;
mod group_essence;
mod group_events;
mod group_files;
mod group_folders;
mod group_membership;
mod group_requests;
mod interactions;
mod smoke;
