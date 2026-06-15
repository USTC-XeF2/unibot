use super::*;

#[sqlx::test]
async fn delete_group_album_is_scoped_to_group(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
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

    repo.create_group_album(&GroupAlbumEntity {
        album_id: "album-a".to_string(),
        group_id: "20001".to_string(),
        name: "Album A".to_string(),
        cover_url: None,
        photo_count: 0,
        created_at: 100,
        updated_at: 100,
    })
    .await?;
    repo.create_group_album(&GroupAlbumEntity {
        album_id: "album-b".to_string(),
        group_id: "20002".to_string(),
        name: "Album B".to_string(),
        cover_url: None,
        photo_count: 0,
        created_at: 100,
        updated_at: 100,
    })
    .await?;

    let deleted = repo.delete_group_album("album-b", "20001").await?;
    assert!(!deleted);
    assert!(repo.get_group_album_by_id("album-b").await?.is_some());

    let deleted = repo.delete_group_album("album-b", "20002").await?;
    assert!(deleted);
    assert!(repo.get_group_album_by_id("album-b").await?.is_none());

    Ok(())
}

#[sqlx::test]
async fn delete_group_photo_refreshes_album_cover(
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
    repo.create_group_album(&GroupAlbumEntity {
        album_id: "album-a".to_string(),
        group_id: "20001".to_string(),
        name: "Album A".to_string(),
        cover_url: None,
        photo_count: 0,
        created_at: 100,
        updated_at: 100,
    })
    .await?;
    repo.create_group_photo(&GroupPhotoEntity {
        photo_id: "photo-1".to_string(),
        album_id: "album-a".to_string(),
        group_id: "20001".to_string(),
        url: "/tmp/photo-1.png".to_string(),
        file_path: Some("groups/20001/files/photo-1.png".to_string()),
        description: None,
        uploader_user_id: "10001".to_string(),
        file_size: Some(1),
        created_at: 100,
    })
    .await?;
    repo.create_group_photo(&GroupPhotoEntity {
        photo_id: "photo-2".to_string(),
        album_id: "album-a".to_string(),
        group_id: "20001".to_string(),
        url: "/tmp/photo-2.png".to_string(),
        file_path: Some("groups/20001/files/photo-2.png".to_string()),
        description: None,
        uploader_user_id: "10001".to_string(),
        file_size: Some(1),
        created_at: 200,
    })
    .await?;
    repo.set_album_cover_if_unset("album-a", "/tmp/photo-1.png")
        .await?;

    let deleted = repo
        .delete_group_photo_and_refresh_cover("photo-1", "20001")
        .await?;

    assert!(deleted);
    let album = repo.get_group_album_by_id("album-a").await?.unwrap();
    assert_eq!(album.cover_url.as_deref(), Some("/tmp/photo-2.png"));

    let deleted = repo
        .delete_group_photo_and_refresh_cover("photo-2", "20001")
        .await?;

    assert!(deleted);
    let album = repo.get_group_album_by_id("album-a").await?.unwrap();
    assert_eq!(album.cover_url, None);

    Ok(())
}
