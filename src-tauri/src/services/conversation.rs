use crate::error::AppResult;
use crate::models::ConversationState;
use crate::persistence::ConversationRepo;
use crate::utils;

#[derive(Clone)]
pub struct ConversationService {
    repo: ConversationRepo,
}

impl ConversationService {
    pub fn new(repo: ConversationRepo) -> Self {
        Self { repo }
    }

    pub async fn set_conversation_pinned(
        &self,
        user_id: String,
        scene: String,
        peer_user_id: Option<String>,
        group_id: Option<String>,
        is_pinned: bool,
    ) -> AppResult<()> {
        let updated_at = utils::now_ts() as i64;
        self.repo
            .upsert_conversation_pinned(
                &user_id,
                &scene,
                peer_user_id.as_deref(),
                group_id.as_deref(),
                is_pinned,
                updated_at,
            )
            .await?;
        Ok(())
    }

    pub async fn set_conversation_muted(
        &self,
        user_id: String,
        scene: String,
        peer_user_id: Option<String>,
        group_id: Option<String>,
        is_muted: bool,
    ) -> AppResult<()> {
        let updated_at = utils::now_ts() as i64;
        self.repo
            .upsert_conversation_muted(
                &user_id,
                &scene,
                peer_user_id.as_deref(),
                group_id.as_deref(),
                is_muted,
                updated_at,
            )
            .await?;
        Ok(())
    }

    pub async fn list_conversation_states(
        &self,
        user_id: String,
    ) -> AppResult<Vec<ConversationState>> {
        let rows = self.repo.list_conversation_states(&user_id).await?;
        Ok(rows)
    }
}
