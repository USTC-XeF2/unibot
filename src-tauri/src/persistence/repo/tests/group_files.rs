use super::*;

#[sqlx::test]
async fn group_file_parent_folder_must_belong_to_same_group(
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
    repo.upsert_group_folder(&GroupFolderEntity {
        folder_id: "folder-b".to_string(),
        group_id: "20002".to_string(),
        parent_folder_id: None,
        folder_name: "Group B Folder".to_string(),
        creator_user_id: "10002".to_string(),
        created_at: 100,
        updated_at: 100,
        file_count: 0,
    })
    .await?;

    let result = repo
        .upsert_group_file(&GroupFileEntity {
            file_id: "file-a".to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: Some("folder-b".to_string()),
            file_name: "cross.txt".to_string(),
            file_size: 1,
            file_hash: None,
            uploader_user_id: "10001".to_string(),
            uploaded_at: 200,
            expire_at: None,
            download_count: 0,
            file_path: Some("groups/20001/files/file-a_cross.txt".to_string()),
        })
        .await;

    assert!(result.is_err());
    assert!(
        repo.list_group_files("20001", Some("folder-b"))
            .await?
            .is_empty()
    );

    Ok(())
}
