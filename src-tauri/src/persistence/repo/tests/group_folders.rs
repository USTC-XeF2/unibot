use super::*;

#[sqlx::test]
async fn upsert_group_folder_does_not_update_other_group(
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
        folder_id: "folder-shared".to_string(),
        group_id: "20002".to_string(),
        parent_folder_id: None,
        folder_name: "original".to_string(),
        creator_user_id: "10002".to_string(),
        created_at: 100,
        updated_at: 100,
        file_count: 0,
    })
    .await?;

    repo.upsert_group_folder(&GroupFolderEntity {
        folder_id: "folder-shared".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: None,
        folder_name: "cross-group overwrite".to_string(),
        creator_user_id: "10001".to_string(),
        created_at: 200,
        updated_at: 200,
        file_count: 0,
    })
    .await?;

    let group_b_folders = repo.list_group_folders("20002").await?;
    assert_eq!(group_b_folders.len(), 1);
    assert_eq!(group_b_folders[0].folder_name, "original");
    assert!(repo.list_group_folders("20001").await?.is_empty());

    Ok(())
}

#[sqlx::test]
async fn group_folder_parent_must_exist_and_belong_to_same_group(
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

    let missing_parent = repo
        .upsert_group_folder(&GroupFolderEntity {
            folder_id: "folder-a".to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: Some("missing-folder".to_string()),
            folder_name: "Missing Parent".to_string(),
            creator_user_id: "10001".to_string(),
            created_at: 200,
            updated_at: 200,
            file_count: 0,
        })
        .await;
    assert!(missing_parent.is_err());

    let cross_group_parent = repo
        .upsert_group_folder(&GroupFolderEntity {
            folder_id: "folder-a".to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: Some("folder-b".to_string()),
            folder_name: "Cross Parent".to_string(),
            creator_user_id: "10001".to_string(),
            created_at: 200,
            updated_at: 200,
            file_count: 0,
        })
        .await;
    assert!(cross_group_parent.is_err());
    assert!(repo.list_group_folders("20001").await?.is_empty());

    Ok(())
}

#[sqlx::test]
async fn group_folder_parent_cannot_create_cycle(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool);
    repo.upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    repo.upsert_group_folder(&GroupFolderEntity {
        folder_id: "parent".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: None,
        folder_name: "Parent".to_string(),
        creator_user_id: "10001".to_string(),
        created_at: 100,
        updated_at: 100,
        file_count: 0,
    })
    .await?;
    repo.upsert_group_folder(&GroupFolderEntity {
        folder_id: "child".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: Some("parent".to_string()),
        folder_name: "Child".to_string(),
        creator_user_id: "10001".to_string(),
        created_at: 200,
        updated_at: 200,
        file_count: 0,
    })
    .await?;

    let self_parent = repo
        .upsert_group_folder(&GroupFolderEntity {
            folder_id: "parent".to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: Some("parent".to_string()),
            folder_name: "Self Parent".to_string(),
            creator_user_id: "10001".to_string(),
            created_at: 300,
            updated_at: 300,
            file_count: 0,
        })
        .await;
    assert!(self_parent.is_err());

    let descendant_parent = repo
        .upsert_group_folder(&GroupFolderEntity {
            folder_id: "parent".to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: Some("child".to_string()),
            folder_name: "Cycle".to_string(),
            creator_user_id: "10001".to_string(),
            created_at: 300,
            updated_at: 300,
            file_count: 0,
        })
        .await;
    assert!(descendant_parent.is_err());

    let parent = repo.get_group_folder_by_id("parent").await?.unwrap();
    assert_eq!(parent.parent_folder_id, None);
    assert_eq!(parent.folder_name, "Parent");

    Ok(())
}

#[sqlx::test]
async fn delete_group_folder_removes_descendants_and_files(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool);
    repo.upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    repo.upsert_group_folder(&GroupFolderEntity {
        folder_id: "parent".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: None,
        folder_name: "Parent".to_string(),
        creator_user_id: "10001".to_string(),
        created_at: 100,
        updated_at: 100,
        file_count: 0,
    })
    .await?;
    repo.upsert_group_folder(&GroupFolderEntity {
        folder_id: "child".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: Some("parent".to_string()),
        folder_name: "Child".to_string(),
        creator_user_id: "10001".to_string(),
        created_at: 200,
        updated_at: 200,
        file_count: 0,
    })
    .await?;
    repo.upsert_group_file(&GroupFileEntity {
        file_id: "file-parent".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: Some("parent".to_string()),
        file_name: "parent.txt".to_string(),
        file_size: 7,
        file_hash: None,
        uploader_user_id: "10001".to_string(),
        uploaded_at: 300,
        expire_at: None,
        download_count: 0,
        file_path: Some("groups/20001/files/file-parent_parent.txt".to_string()),
    })
    .await?;
    repo.upsert_group_file(&GroupFileEntity {
        file_id: "file-child".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: Some("child".to_string()),
        file_name: "child.txt".to_string(),
        file_size: 5,
        file_hash: None,
        uploader_user_id: "10001".to_string(),
        uploaded_at: 400,
        expire_at: None,
        download_count: 0,
        file_path: Some("groups/20001/files/file-child_child.txt".to_string()),
    })
    .await?;

    let deleted = repo.delete_group_folder("parent").await?;

    assert!(deleted);
    assert!(repo.get_group_folder_by_id("parent").await?.is_none());
    assert!(repo.get_group_folder_by_id("child").await?.is_none());
    assert!(repo.get_group_file_by_id("file-parent").await?.is_none());
    assert!(repo.get_group_file_by_id("file-child").await?.is_none());

    Ok(())
}

#[sqlx::test]
async fn group_folder_update_requires_creator_or_admin(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Owner"))
        .await?;
    user_repo
        .upsert_user(&make_profile("10002", "Creator"))
        .await?;
    user_repo
        .upsert_user(&make_profile("10003", "Member"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10002", GroupRole::Member))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10003", GroupRole::Member))
        .await?;
    group_repo
        .upsert_group_folder(&GroupFolderEntity {
            folder_id: "folder-1".to_string(),
            group_id: "20001".to_string(),
            parent_folder_id: None,
            folder_name: "Original".to_string(),
            creator_user_id: "10002".to_string(),
            created_at: 100,
            updated_at: 100,
            file_count: 0,
        })
        .await?;

    let core = CoreContainer::new();
    register_test_user(&core, "10001", "Owner");
    register_test_user(&core, "10002", "Creator");
    register_test_user(&core, "10003", "Member");
    let service = GroupService::new(group_repo.clone(), MessageRepo::new(pool));

    let unauthorized = service
        .validate_group_folder_upsert(
            &core,
            &GroupFolderEntity {
                folder_id: "folder-1".to_string(),
                group_id: "20001".to_string(),
                parent_folder_id: None,
                folder_name: "Hijacked".to_string(),
                creator_user_id: "10003".to_string(),
                created_at: 200,
                updated_at: 200,
                file_count: 0,
            },
        )
        .await;

    assert!(unauthorized.is_err());
    assert_eq!(
        group_repo
            .get_group_folder_by_id("folder-1")
            .await?
            .unwrap()
            .folder_name,
        "Original"
    );

    let creator_update = service
        .validate_group_folder_upsert(
            &core,
            &GroupFolderEntity {
                folder_id: "folder-1".to_string(),
                group_id: "20001".to_string(),
                parent_folder_id: None,
                folder_name: "Creator Rename".to_string(),
                creator_user_id: "10002".to_string(),
                created_at: 200,
                updated_at: 200,
                file_count: 0,
            },
        )
        .await;
    assert!(creator_update.is_ok());

    Ok(())
}
