use super::*;

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
