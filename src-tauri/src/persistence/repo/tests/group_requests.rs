use super::*;

#[sqlx::test]
async fn smoke_group_requests(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let repo = GroupRepo::new(pool);
    repo.upsert_group(&make_group("20001", "Test", "10001"))
        .await?;
    repo.upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;

    // Create join request
    let req = repo
        .create_group_request(NewGroupRequestRecord {
            group_id: "20001".to_string(),
            request_type: GroupRequestType::Join,
            initiator_user_id: "10002".to_string(),
            target_user_id: None,
            comment: None,
            created_at: 100,
        })
        .await?;
    assert!(!req.request_id.is_empty());
    assert_eq!(req.state, RequestState::Pending);

    assert!(
        repo.has_pending_group_request("20001", GroupRequestType::Join, "10002", None)
            .await?
    );

    let handled = repo
        .handle_group_request(&req.request_id, RequestState::Accepted, "10001", 200, 200)
        .await?;
    assert!(handled.is_some());
    assert_eq!(handled.unwrap().state, RequestState::Accepted);

    // Accepted join request must populate user_groups
    let user_groups = repo.list_user_groups("10002").await?;
    assert!(user_groups.iter().any(|g| g.group_id == "20001"));

    // member_count must reflect both owner and accepted joiner
    let group = repo.get_group("20001").await?;
    assert_eq!(group.map(|g| g.member_count), Some(2));

    Ok(())
}
