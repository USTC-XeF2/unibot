use super::*;

#[sqlx::test]
async fn smoke_crud_bots_and_debug_sessions(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = BotRepo::new(pool);
    let bot = repo
        .insert_bot("bot_10001", "10001", "Alice Bot", "/tmp/bot.json")
        .await?;
    assert_eq!(bot.bot_id, "bot_10001");
    assert_eq!(bot.runtime_status, "stopped");

    let duplicate = repo
        .insert_bot("bot_duplicate", "10001", "Duplicate", "/tmp/dup.json")
        .await;
    assert!(duplicate.is_err());

    let listed = repo.list_bots().await?;
    assert_eq!(listed.len(), 1);

    let found = repo.find_bot_by_bound_user_id("10001").await?;
    assert!(found.is_some());

    let session = repo
        .start_session("session_1", "bot_10001", "Debug Session")
        .await?;
    assert_eq!(session.bot_id, "bot_10001");
    assert!(session.ended_at.is_none());

    let running = repo.get_bot_by_id("bot_10001").await?.unwrap();
    assert_eq!(running.runtime_status, "running");
    assert_eq!(repo.get_online_bot_count().await?, 1);

    repo.stop_active_sessions("bot_10001").await?;
    let stopped = repo.get_bot_by_id("bot_10001").await?.unwrap();
    assert_eq!(stopped.runtime_status, "stopped");
    assert_eq!(repo.get_online_bot_count().await?, 0);

    let sessions = repo.list_sessions_by_bot("bot_10001").await?;
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].ended_at.is_some());

    assert!(repo.delete_bot_with_sessions("bot_10001").await?.is_some());
    assert!(repo.get_bot_by_id("bot_10001").await?.is_none());

    Ok(())
}

#[sqlx::test]
async fn delete_bot_with_sessions_removes_running_bot_atomically(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = BotRepo::new(pool);
    repo.insert_bot("bot_10001", "10001", "Alice Bot", "/tmp/bot.json")
        .await?;
    repo.start_session("session_1", "bot_10001", "Debug Session")
        .await?;

    let deleted = repo
        .delete_bot_with_sessions("bot_10001")
        .await?
        .expect("running bot should be deleted");

    assert_eq!(deleted.bot_id, "bot_10001");
    assert_eq!(deleted.config_path, "/tmp/bot.json");
    assert!(repo.get_bot_by_id("bot_10001").await?.is_none());
    assert_eq!(repo.get_online_bot_count().await?, 0);
    assert!(repo.delete_bot_with_sessions("bot_10001").await?.is_none());

    Ok(())
}

#[sqlx::test]
async fn bot_session_lifecycle_updates_status_atomically(
    pool: sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;

    let repo = BotRepo::new(pool);
    repo.insert_bot("bot_10001", "10001", "Alice Bot", "/tmp/bot.json")
        .await?;

    let session = repo
        .start_session("session_1", "bot_10001", "Debug Session")
        .await?;
    assert_eq!(session.session_id, "session_1");

    let running = repo.get_bot_by_id("bot_10001").await?.unwrap();
    assert_eq!(running.runtime_status, "running");

    let duplicate = repo
        .start_session("session_2", "bot_10001", "Duplicate Session")
        .await;
    assert!(duplicate.is_err());

    repo.stop_active_sessions("bot_10001").await?;
    let stopped = repo.get_bot_by_id("bot_10001").await?.unwrap();
    assert_eq!(stopped.runtime_status, "stopped");

    Ok(())
}
