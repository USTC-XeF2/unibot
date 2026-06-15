use super::*;

#[sqlx::test]
async fn upsert_announcement_does_not_update_other_group(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let repo = GroupRepo::new(pool);
    repo.upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    repo.upsert_group(&make_group("20002", "Group B", "10002"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    repo.upsert_group_member(&make_member("20002", "10002", GroupRole::Owner))
        .await?;

    repo.upsert_announcement(&GroupAnnouncementEntity {
        announcement_id: "ann-shared".to_string(),
        group_id: "20002".to_string(),
        sender_user_id: "10002".to_string(),
        content: "original".to_string(),
        image_url: None,
        created_at: 100,
        updated_at: 100,
    })
    .await?;

    repo.upsert_announcement(&GroupAnnouncementEntity {
        announcement_id: "ann-shared".to_string(),
        group_id: "20001".to_string(),
        sender_user_id: "10001".to_string(),
        content: "cross-group overwrite".to_string(),
        image_url: None,
        created_at: 200,
        updated_at: 200,
    })
    .await?;

    let group_b_announcements = repo.list_announcements("20002").await?;
    assert_eq!(group_b_announcements.len(), 1);
    assert_eq!(group_b_announcements[0].content, "original");
    assert!(repo.list_announcements("20001").await?.is_empty());

    Ok(())
}

#[sqlx::test]
async fn delete_announcement_reports_missing_record(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Owner"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    let deleted = group_repo
        .delete_announcement_or_not_found("20001", "missing-announcement")
        .await;

    assert!(deleted.is_err());

    Ok(())
}
