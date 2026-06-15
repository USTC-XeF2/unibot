use super::*;
use std::path::PathBuf;

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

#[sqlx::test]
async fn increment_group_file_download_count_is_atomic(
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
    repo.upsert_group_file(&GroupFileEntity {
        file_id: "file-a".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: None,
        file_name: "report.txt".to_string(),
        file_size: 7,
        file_hash: None,
        uploader_user_id: "10001".to_string(),
        uploaded_at: 200,
        expire_at: None,
        download_count: 0,
        file_path: Some("groups/20001/files/file-a_report.txt".to_string()),
    })
    .await?;

    assert!(repo.increment_group_file_download_count("file-a").await?);
    assert_eq!(
        repo.get_group_file_by_id("file-a")
            .await?
            .unwrap()
            .download_count,
        1
    );

    Ok(())
}

#[sqlx::test]
async fn download_group_file_copies_file_then_increments_count(
    pool: sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool.clone());
    repo.upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    repo.upsert_group_file(&GroupFileEntity {
        file_id: "file-a".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: None,
        file_name: "report.txt".to_string(),
        file_size: 7,
        file_hash: None,
        uploader_user_id: "10001".to_string(),
        uploaded_at: 200,
        expire_at: None,
        download_count: 0,
        file_path: Some("groups/20001/files/file-a_report.txt".to_string()),
    })
    .await?;

    let app_data_dir =
        std::env::temp_dir().join(format!("unibot-download-service-{}", uuid::Uuid::new_v4()));
    let stored_path = app_data_dir.join("groups/20001/files/file-a_report.txt");
    tokio::fs::create_dir_all(stored_path.parent().unwrap()).await?;
    tokio::fs::write(&stored_path, b"report").await?;
    let destination =
        std::env::temp_dir().join(format!("unibot-downloaded-{}.txt", uuid::Uuid::new_v4()));

    let service = GroupService::new(repo.clone(), MessageRepo::new(pool));
    let downloaded = service
        .download_group_file(
            "10001".to_string(),
            "20001".to_string(),
            "file-a".to_string(),
            destination.clone(),
            app_data_dir.clone(),
        )
        .await?;

    assert_eq!(downloaded, destination.to_string_lossy());
    assert_eq!(tokio::fs::read(&destination).await?, b"report");
    assert_eq!(
        repo.get_group_file_by_id("file-a")
            .await?
            .unwrap()
            .download_count,
        1
    );

    let _ = tokio::fs::remove_dir_all(app_data_dir).await;
    let _ = tokio::fs::remove_file(destination).await;
    Ok(())
}

#[sqlx::test]
async fn download_group_file_does_not_increment_count_when_copy_fails(
    pool: sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = GroupRepo::new(pool.clone());
    repo.upsert_group(&make_group("20001", "Group A", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    repo.upsert_group_file(&GroupFileEntity {
        file_id: "file-a".to_string(),
        group_id: "20001".to_string(),
        parent_folder_id: None,
        file_name: "report.txt".to_string(),
        file_size: 7,
        file_hash: None,
        uploader_user_id: "10001".to_string(),
        uploaded_at: 200,
        expire_at: None,
        download_count: 0,
        file_path: Some("groups/20001/files/file-a_report.txt".to_string()),
    })
    .await?;

    let app_data_dir =
        std::env::temp_dir().join(format!("unibot-download-service-{}", uuid::Uuid::new_v4()));
    let stored_path = app_data_dir.join("groups/20001/files/file-a_report.txt");
    tokio::fs::create_dir_all(stored_path.parent().unwrap()).await?;
    tokio::fs::write(&stored_path, b"report").await?;

    let service = GroupService::new(repo.clone(), MessageRepo::new(pool));
    let result = service
        .download_group_file(
            "10001".to_string(),
            "20001".to_string(),
            "file-a".to_string(),
            PathBuf::from("relative.txt"),
            app_data_dir.clone(),
        )
        .await;

    assert!(result.is_err());
    assert_eq!(
        repo.get_group_file_by_id("file-a")
            .await?
            .unwrap()
            .download_count,
        0
    );

    let _ = tokio::fs::remove_dir_all(app_data_dir).await;
    Ok(())
}
