use super::*;

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

    // Duplicate upsert must not soft-delete — account_status stays active
    let alice_v2 = crate::models::UserProfile {
        nickname: "Alice2".to_string(),
        ..alice
    };
    repo.upsert_user(&alice_v2).await?;
    let after_dup = repo.get_user_by_id("10001").await?;
    assert!(after_dup.is_some());
    let after_dup = after_dup.unwrap();
    assert_eq!(after_dup.nickname, "Alice2");
    assert_eq!(
        after_dup.account_status,
        crate::models::AccountStatus::Active
    );

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
        .handle_friend_request_for_target(&created.request_id, RequestState::Accepted, "10002", 200)
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

    // Remove member — must also remove user_groups entry
    repo.remove_group_member("20001", "10002").await?;
    let after = repo.list_group_members("20001").await?;
    assert_eq!(after.len(), 1);
    let after_user_groups = repo.list_user_groups("10002").await?;
    assert!(!after_user_groups.iter().any(|g| g.group_id == "20001"));

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
            bot_id: None,
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
            bot_id: Some("bot_10001".to_string()),
        })
        .await?;
    assert!(!grp_msg.id.is_empty());
    assert_eq!(grp_msg.bot_id.as_deref(), Some("bot_10001"));

    let essence = group_repo
        .create_group_essence_message("20001", &grp_msg.id, "10001", "10001", true, 300)
        .await?;
    assert!(essence.is_set);
    assert_eq!(essence.message_id, grp_msg.id);
    assert_eq!(essence.operator_user_id, "10001");

    let essences = group_repo.list_group_essence_messages("20001").await?;
    assert_eq!(essences.len(), 1);
    assert_eq!(essences[0].message_id, grp_msg.id);

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
    assert_eq!(grp_history[0].bot_id.as_deref(), Some("bot_10001"));

    assert_eq!(msg_repo.get_message_count().await?, 2);

    // Recall
    let recalled = msg_repo
        .mark_message_recalled(&priv_msg.id, "10001")
        .await?;
    assert!(recalled.is_some());
    assert!(recalled.unwrap().is_recalled);

    Ok(())
}
