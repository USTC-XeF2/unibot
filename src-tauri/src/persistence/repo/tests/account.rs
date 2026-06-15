use super::*;

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
            bot_id: None,
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
