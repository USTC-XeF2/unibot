use super::*;
use crate::models::InternalEvent;

fn received_group_member_left(
    receiver: &mut tokio::sync::broadcast::Receiver<InternalEvent>,
    target_user_id: &str,
) -> bool {
    while let Ok(event) = receiver.try_recv() {
        if matches!(
            event,
            InternalEvent::GroupMemberLeft {
                target_user_id: ref target,
                ..
            } if target == target_user_id
        ) {
            return true;
        }
    }
    false
}

#[sqlx::test]
async fn kick_emits_group_member_left_to_removed_user(
    pool: sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Owner"))
        .await?;
    user_repo
        .upsert_user(&make_profile("10002", "Member"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Group", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10002", GroupRole::Member))
        .await?;

    let core = CoreContainer::new();
    register_test_user(&core, "10001", "Owner");
    register_test_user(&core, "10002", "Member");
    let mut removed_events = core.user_context("10002").unwrap().subscribe_events();
    let service = GroupService::new(group_repo, MessageRepo::new(pool));

    service
        .kick_group_member(
            &core,
            "10001".to_string(),
            "20001".to_string(),
            "10002".to_string(),
        )
        .await?;

    assert!(received_group_member_left(&mut removed_events, "10002"));
    Ok(())
}

#[sqlx::test]
async fn dissolve_emits_group_member_left_to_every_member(
    pool: sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Owner"))
        .await?;
    user_repo
        .upsert_user(&make_profile("10002", "Member"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Group", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10002", GroupRole::Member))
        .await?;

    let core = CoreContainer::new();
    register_test_user(&core, "10001", "Owner");
    register_test_user(&core, "10002", "Member");
    let mut owner_events = core.user_context("10001").unwrap().subscribe_events();
    let mut member_events = core.user_context("10002").unwrap().subscribe_events();
    let service = GroupService::new(group_repo, MessageRepo::new(pool));

    service
        .dissolve_group(&core, "10001".to_string(), "20001".to_string())
        .await?;

    assert!(received_group_member_left(&mut owner_events, "10001"));
    assert!(received_group_member_left(&mut member_events, "10002"));
    Ok(())
}

#[sqlx::test]
async fn leave_emits_group_member_left_to_leaving_user(
    pool: sqlx::SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Owner"))
        .await?;
    user_repo
        .upsert_user(&make_profile("10002", "Member"))
        .await?;

    let group_repo = GroupRepo::new(pool.clone());
    group_repo
        .upsert_group(&make_group("20001", "Group", "10001"))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10001", GroupRole::Owner))
        .await?;
    group_repo
        .upsert_group_member(&make_member("20001", "10002", GroupRole::Member))
        .await?;

    let core = CoreContainer::new();
    register_test_user(&core, "10001", "Owner");
    register_test_user(&core, "10002", "Member");
    let mut leaving_events = core.user_context("10002").unwrap().subscribe_events();
    let service = GroupService::new(group_repo, MessageRepo::new(pool));

    service
        .leave_group(&core, "10002".to_string(), "20001".to_string())
        .await?;

    assert!(received_group_member_left(&mut leaving_events, "10002"));
    Ok(())
}
