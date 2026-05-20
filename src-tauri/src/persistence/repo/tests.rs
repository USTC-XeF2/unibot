use crate::models::{GroupMemberProfile, GroupProfile, GroupRequestType, GroupRole, RequestState};
use crate::persistence::{
    GroupRepo, InteractionRepo, MessageRepo, NewFriendRequestRecord, NewGroupEventRecord,
    NewGroupRequestRecord, NewMessageReactionRecord, NewMessageRecord, NewPokeRecord, UserRepo,
    migrator,
};

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

#[sqlx::test]
async fn smoke_crud_users(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let repo = UserRepo::new(pool);
    let alice = make_profile("10001", "Alice");
    let bob = make_profile("10002", "Bob");

    repo.upsert_user(&alice).await?;
    repo.upsert_user(&bob).await?;

    let users = repo.list_users().await?;
    assert_eq!(users.len(), 2);

    let got = repo.get_user_by_id("10001").await?;
    assert!(got.is_some());
    assert_eq!(got.unwrap().nickname, "Alice");

    Ok(())
}

#[sqlx::test]
async fn smoke_crud_friends(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let repo = UserRepo::new(pool);
    repo.upsert_user(&make_profile("10001", "Alice")).await?;
    repo.upsert_user(&make_profile("10002", "Bob")).await?;

    assert!(!repo.are_friends("10001", "10002").await?);

    let created = repo
        .create_friend_request(NewFriendRequestRecord {
            initiator_user_id: "10001".to_string(),
            target_user_id: "10002".to_string(),
            comment: "hello".to_string(),
            created_at: 100,
        })
        .await?;
    assert!(!created.request_id.is_empty());
    assert_eq!(created.state, RequestState::Pending);

    assert!(
        repo.has_pending_friend_request_between("10001", "10002")
            .await?
    );

    let handled = repo
        .handle_friend_request_for_target(
            &created.request_id,
            RequestState::Accepted,
            "10002",
            "10002",
            200,
        )
        .await?;
    assert!(handled.is_some());
    assert_eq!(handled.unwrap().state, RequestState::Accepted);

    assert!(repo.are_friends("10001", "10002").await?);

    let friends = repo.list_friends("10001").await?;
    assert_eq!(friends.len(), 1);
    assert_eq!(friends[0].friend_user_id, "10002");

    assert!(repo.remove_friendship_pair("10001", "10002").await?);
    assert!(!repo.are_friends("10001", "10002").await?);

    Ok(())
}

#[sqlx::test]
async fn smoke_crud_groups(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;
    user_repo
        .upsert_user(&make_profile("10003", "Carol"))
        .await?;

    let repo = GroupRepo::new(pool);
    let group = make_group("20001", "Test Group", "10001");
    repo.upsert_group(&group).await?;

    let got = repo.get_group("20001").await?;
    assert!(got.is_some());
    assert_eq!(got.unwrap().group_name, "Test Group");

    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10002", GroupRole::Member))
        .await?;

    let members = repo.list_group_members("20001").await?;
    assert_eq!(members.len(), 2);

    let owner_member = repo.get_group_member("20001", "10001").await?;
    assert!(owner_member.is_some());
    assert_eq!(owner_member.unwrap().role, GroupRole::Owner);

    // Update role
    let updated = repo
        .update_group_member_role("20001", "10002", GroupRole::Admin)
        .await?;
    assert!(updated.is_some());
    assert_eq!(updated.unwrap().role, GroupRole::Admin);

    // Whole-mute
    let mute = repo
        .set_group_whole_mute("20001", true, Some(5000), "10001", 100)
        .await?;
    assert!(mute.muted);

    let got_mute = repo.get_group_whole_mute("20001").await?;
    assert!(got_mute.is_some());
    assert!(got_mute.unwrap().muted);

    // Remove member
    repo.remove_group_member("20001", "10002").await?;
    let after = repo.list_group_members("20001").await?;
    assert_eq!(after.len(), 1);

    // `upsert_group_member` now writes user_groups alongside group_members
    let user_groups = repo.list_user_groups("10001").await?;
    assert!(!user_groups.is_empty());

    Ok(())
}

#[sqlx::test]
async fn smoke_crud_messages(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Test", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10002", GroupRole::Member))
        .await?;

    let msg_repo = MessageRepo::new(pool);

    // Private message
    let priv_msg = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "private".to_string(),
            source_id: "10002".to_string(),
            content_json: "[]".to_string(),
            quoted_message_id: None,
            created_at: 100,
        })
        .await?;
    assert!(!priv_msg.id.is_empty());

    // Group message
    let grp_msg = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "group".to_string(),
            source_id: "20001".to_string(),
            content_json: "[]".to_string(),
            quoted_message_id: None,
            created_at: 200,
        })
        .await?;
    assert!(!grp_msg.id.is_empty());

    // List private history
    let priv_history = msg_repo
        .list_messages("10001", "private", "10002", 50)
        .await?;
    assert!(!priv_history.is_empty());

    // List group history
    let grp_history = msg_repo
        .list_messages("10001", "group", "20001", 50)
        .await?;
    assert!(!grp_history.is_empty());

    // Recall
    let recalled = msg_repo
        .mark_message_recalled(&priv_msg.id, "10001")
        .await?;
    assert!(recalled.is_some());
    assert!(recalled.unwrap().is_recalled);

    Ok(())
}

#[sqlx::test]
async fn smoke_crud_interactions(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let msg_repo = MessageRepo::new(pool.clone());
    let priv_msg = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "private".to_string(),
            source_id: "10002".to_string(),
            content_json: "[]".to_string(),
            quoted_message_id: None,
            created_at: 100,
        })
        .await?;

    let interaction_repo = InteractionRepo::new(pool);

    // Reaction
    let reaction = interaction_repo
        .insert_message_reaction(NewMessageReactionRecord {
            message_id: priv_msg.id.clone(),
            operator_user_id: "10002".to_string(),
            face_id: "face_001".to_string(),
            is_add: true,
            created_at: 200,
        })
        .await?;
    assert!(!reaction.reaction_id.is_empty());
    assert_eq!(reaction.face_id, "face_001");

    // Poke
    let poke = interaction_repo
        .insert_poke(NewPokeRecord {
            source_type: "private".to_string(),
            source_id: "10001".to_string(),
            sender_user_id: "10002".to_string(),
            target_user_id: "10001".to_string(),
            created_at: 300,
        })
        .await?;
    assert!(!poke.poke_id.is_empty());

    let pokes = interaction_repo
        .list_pokes("10001", "private", "10002", 50)
        .await?;
    assert!(!pokes.is_empty());

    Ok(())
}

#[sqlx::test]
async fn smoke_group_requests(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let repo = GroupRepo::new(pool);
    repo.upsert_group(&make_group("20001", "Test", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    // Create join request
    let req = repo
        .create_group_request(NewGroupRequestRecord {
            group_id: "20001".to_string(),
            request_type: GroupRequestType::Join,
            initiator_user_id: "10002".to_string(),
            target_user_id: None,
            comment: None,
            created_at: 100,
        })
        .await?;
    assert!(!req.request_id.is_empty());
    assert_eq!(req.state, RequestState::Pending);

    assert!(
        repo.has_pending_group_request("20001", GroupRequestType::Join, "10002", None)
            .await?
    );

    let handled = repo
        .handle_group_request(&req.request_id, RequestState::Accepted, "10001", 200, 200)
        .await?;
    assert!(handled.is_some());
    assert_eq!(handled.unwrap().state, RequestState::Accepted);

    Ok(())
}

#[sqlx::test]
async fn smoke_account_deletion_retains_rows(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Test Group", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    let msg_repo = MessageRepo::new(pool.clone());
    let _grp_msg = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "group".to_string(),
            source_id: "20001".to_string(),
            content_json: "[]".to_string(),
            quoted_message_id: None,
            created_at: 100,
        })
        .await?;

    // Delete account
    let deleted = user_repo.delete_user("10001").await?;
    assert!(deleted);

    // User row retained with deleted status
    let user_row = user_repo.get_user_by_id("10001").await?;
    assert!(user_row.is_some());
    assert_eq!(
        user_row.unwrap().account_status,
        crate::models::AccountStatus::Deleted
    );

    // Owned group dissolved
    let group = group_repo.get_group("20001").await?;
    assert!(group.is_some());
    assert_eq!(
        group.unwrap().group_status,
        crate::models::GroupStatus::Dissolved
    );

    // Historical group message still references group_id
    let history = msg_repo
        .list_messages("10001", "group", "20001", 50)
        .await?;
    assert!(!history.is_empty());

    Ok(())
}

#[sqlx::test]
async fn smoke_group_events(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool);
    repo.upsert_group(&make_group("20001", "Test", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    repo.insert_group_event(NewGroupEventRecord {
        group_id: "20001".to_string(),
        payload: serde_json::json!({"type": "member_joined", "user_id": "10002"}).to_string(),
        created_at: 100,
    })
    .await?;

    let events = repo.list_group_events("20001", 50).await?;
    assert!(!events.is_empty());
    assert_eq!(events[0].group_id, "20001");

    Ok(())
}
