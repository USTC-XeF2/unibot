use super::*;

#[sqlx::test]
async fn smoke_crud_interactions(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    setup(&pool).await;

    let user_repo = UserRepo::new(pool.clone());
    user_repo
        .upsert_user(&make_profile("10001", "Alice"))
        .await?;
    user_repo.upsert_user(&make_profile("10002", "Bob")).await?;

    let msg_repo = MessageRepo::new(pool.clone());
    let priv_msg = msg_repo
        .insert_message(NewMessageRecord {
            owner_user_id: "10001".to_string(),
            sender_user_id: "10001".to_string(),
            source_type: "private".to_string(),
            source_id: "10002".to_string(),
            content_json: "[]".to_string(),
            quoted_message_id: None,
            created_at: 100,
            bot_id: None,
        })
        .await?;

    let interaction_repo = InteractionRepo::new(pool);

    // Reaction
    let reaction = interaction_repo
        .insert_message_reaction(NewMessageReactionRecord {
            message_id: priv_msg.id.clone(),
            operator_user_id: "10002".to_string(),
            face_id: "face_001".to_string(),
            is_add: true,
            created_at: 200,
        })
        .await?;
    assert!(!reaction.reaction_id.is_empty());
    assert_eq!(reaction.face_id, "face_001");

    // Poke
    let poke = interaction_repo
        .insert_poke(NewPokeRecord {
            source_type: "private".to_string(),
            source_id: "10001".to_string(),
            sender_user_id: "10002".to_string(),
            target_user_id: "10001".to_string(),
            created_at: 300,
        })
        .await?;
    assert!(!poke.poke_id.is_empty());

    let pokes = interaction_repo
        .list_pokes("10001", "private", "10002", 50)
        .await?;
    assert!(!pokes.is_empty());

    Ok(())
}
