use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{InternalEvent, MessageReactionEntity, MessageSource, PokeEntity};
use crate::persistence::{
    GroupRepo, InteractionRepo, MessageRepo, NewMessageReactionRecord, NewPokeRecord,
};
use crate::utils::{emit_to_users, now_ts, recipients_for_source};

#[derive(Clone)]
pub struct InteractionService {
    repo: InteractionRepo,
    message_repo: MessageRepo,
    group_repo: GroupRepo,
}

impl InteractionService {
    pub fn new(repo: InteractionRepo, message_repo: MessageRepo, group_repo: GroupRepo) -> Self {
        Self {
            repo,
            message_repo,
            group_repo,
        }
    }

    pub async fn react_to_message(
        &self,
        core: &CoreContainer,
        user_id: u64,
        message_id: i64,
        face_id: String,
        is_add: bool,
    ) -> AppResult<MessageReactionEntity> {
        core.require_user_context(user_id)?;

        let message = self
            .message_repo
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("message {} not found", message_id)))?;

        let record = NewMessageReactionRecord {
            message_id,
            source_type: message.source_type,
            source_id: message.source_id,
            operator_user_id: user_id,
            face_id,
            is_add,
            created_at: now_ts(),
        };

        let reaction = self.repo.insert_message_reaction(record).await?;

        let recipients = recipients_for_source(
            core,
            &self.group_repo,
            &reaction.source,
            user_id,
            Some(message.sender_user_id),
        )
        .await;
        let event = InternalEvent::MessageReaction {
            reaction_id: reaction.reaction_id,
            message_id: reaction.message_id,
            source: reaction.source.clone(),
            operator_user_id: reaction.operator_user_id,
            face_id: reaction.face_id.clone(),
            is_add: reaction.is_add,
            time: reaction.created_at,
        };
        emit_to_users(core, recipients, event);

        Ok(reaction)
    }

    pub async fn poke(
        &self,
        core: &CoreContainer,
        user_id: u64,
        source: MessageSource,
        target_user_id: u64,
    ) -> AppResult<PokeEntity> {
        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let (source_type, source_id) = source.to_db_parts();
        let record = NewPokeRecord {
            source_type: source_type.to_string(),
            source_id,
            sender_user_id: user_id,
            target_user_id,
            created_at: now_ts(),
        };

        let poke = self.repo.insert_poke(record).await?;

        let recipients = recipients_for_source(
            core,
            &self.group_repo,
            &poke.source,
            poke.sender_user_id,
            Some(poke.target_user_id),
        )
        .await;
        let event = InternalEvent::Poke {
            poke_id: poke.poke_id,
            source: poke.source.clone(),
            sender_user_id: poke.sender_user_id,
            target_user_id: poke.target_user_id,
            time: poke.created_at,
        };
        emit_to_users(core, recipients, event);

        Ok(poke)
    }

    pub async fn list_poke_history(
        &self,
        user_id: u64,
        source: MessageSource,
        limit: usize,
    ) -> AppResult<Vec<PokeEntity>> {
        let limit_i64 =
            i64::try_from(limit).map_err(|_| AppError::validation("limit is too large"))?;
        let (source_type, source_id) = source.to_db_parts();
        self.repo
            .list_pokes(user_id, source_type, source_id, limit_i64)
            .await
            .map_err(Into::into)
    }
}
