use super::*;

#[sqlx::test]
async fn group_category_names_are_unique_when_creating_and_renaming(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool);
    let first = repo.create_group_category("10001", "项目组").await?;
    let second = repo.create_group_category("10001", "生活").await?;

    assert!(repo.create_group_category("10001", "项目组").await.is_err());
    assert!(
        repo.rename_group_category("10001", &second.category_id, "项目组")
            .await
            .is_err()
    );
    assert_eq!(
        repo.get_group_category_by_id(&first.category_id)
            .await?
            .unwrap()
            .name,
        "项目组"
    );

    Ok(())
}

#[sqlx::test]
async fn friend_category_names_are_unique_when_creating_and_renaming(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let repo = UserRepo::new(pool);
    repo.upsert_user(&make_profile("10001", "Alice")).await?;

    let first = repo.create_friend_category("10001", "同学").await?;
    let second = repo.create_friend_category("10001", "同事").await?;

    assert!(repo.create_friend_category("10001", "同学").await.is_err());
    assert!(
        repo.rename_friend_category("10001", &second.category_id, "同学")
            .await
            .is_err()
    );
    assert_eq!(
        repo.get_friend_category_by_id(&first.category_id)
            .await?
            .unwrap()
            .name,
        "同学"
    );

    Ok(())
}

#[sqlx::test]
async fn default_group_category_can_be_renamed_and_deleted(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool.clone());
    let service = GroupService::new(repo.clone(), MessageRepo::new(pool));
    let default_category_id = "10001:group:default".to_string();

    let renamed = service
        .rename_group_category(
            "10001".to_string(),
            default_category_id.clone(),
            "常用群聊".to_string(),
        )
        .await
        .expect("default group category should be renameable");
    assert_eq!(renamed.name, "常用群聊");

    service
        .delete_group_category("10001".to_string(), default_category_id.clone())
        .await
        .expect("default group category should be deleteable");
    assert!(
        repo.get_group_category_by_id(&default_category_id)
            .await?
            .is_none()
    );

    Ok(())
}

#[sqlx::test]
async fn default_friend_category_can_be_renamed_and_deleted(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let repo = UserRepo::new(pool.clone());
    repo.upsert_user(&make_profile("10001", "Alice")).await?;
    repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let service = crate::services::UserService::new(repo.clone());
    let default_category_id = "10001:friend:default".to_string();

    sqlx::query(
        r#"
        INSERT INTO friendships (owner_user_id, friend_user_id, friend_category_id, created_at)
        VALUES ('10001', '10002', ?1, 1)
        "#,
    )
    .bind(&default_category_id)
    .execute(&pool)
    .await?;

    let renamed = service
        .rename_friend_category(
            "10001".to_string(),
            default_category_id.clone(),
            "熟人".to_string(),
        )
        .await
        .expect("default friend category should be renameable");
    assert_eq!(renamed.name, "熟人");

    service
        .delete_friend_category("10001".to_string(), default_category_id.clone())
        .await
        .expect("default friend category should be deleteable");
    assert!(
        repo.get_friend_category_by_id(&default_category_id)
            .await?
            .is_none()
    );
    assert_eq!(repo.list_friendships("10001").await?[0].category_id, None);

    Ok(())
}
