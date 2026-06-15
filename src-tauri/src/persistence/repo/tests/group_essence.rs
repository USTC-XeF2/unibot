use super::*;

#[sqlx::test]
async fn list_group_essence_messages_keeps_record_after_message_delete(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Test", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    let msg_repo = MessageRepo::new(pool.clone());
    let message = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "group".to_string(),
            source_id: "20001".to_string(),
            content_json: r#"[{"type":"text","data":{"text":"important"}}]"#.to_string(),
            quoted_message_id: None,
            created_at: 100,
            bot_id: None,
        })
        .await?;

    let essence = group_repo
        .create_group_essence_message("20001", &message.id, "10001", "10001", true, 200)
        .await?;

    sqlx::query("DELETE FROM messages WHERE message_id = ?1")
        .bind(&message.id)
        .execute(&pool)
        .await?;

    let essences = group_repo.list_group_essence_messages("20001").await?;
    assert_eq!(essences.len(), 1);
    assert_eq!(essences[0].sender_user_id, "10001");
    assert!(essences[0].content.is_empty());

    let removed = group_repo
        .delete_group_essence_message("20001", &essence.essence_id)
        .await?
        .unwrap();
    assert_eq!(removed.essence_id, essence.essence_id);
    assert_eq!(removed.message_id, "");
    assert!(!removed.is_set);
    assert!(
        group_repo
            .list_group_essence_messages("20001")
            .await?
            .is_empty()
    );

    Ok(())
}

#[sqlx::test]
async fn delete_group_essence_message_works_after_message_recall(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Test", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    let msg_repo = MessageRepo::new(pool);
    let message = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "group".to_string(),
            source_id: "20001".to_string(),
            content_json: r#"[{"type":"text","data":{"text":"important"}}]"#.to_string(),
            quoted_message_id: None,
            created_at: 100,
            bot_id: None,
        })
        .await?;
    let essence = group_repo
        .create_group_essence_message("20001", &message.id, "10001", "10001", true, 200)
        .await?;
    msg_repo.mark_message_recalled(&message.id, "10001").await?;

    let removed = group_repo
        .delete_group_essence_message("20001", &essence.essence_id)
        .await?
        .unwrap();

    assert_eq!(removed.essence_id, essence.essence_id);
    assert_eq!(removed.message_id, message.id);
    assert!(!removed.is_set);
    assert!(
        group_repo
            .list_group_essence_messages("20001")
            .await?
            .is_empty()
    );

    Ok(())
}
