use serde::Serialize;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupRole, InternalEvent, MessageEntity, MessageRecallInfo, MessageSegment, MessageSource,
};
use crate::persistence::{GroupRepo, MessageRecord, MessageRepo, NewMessageRecord};
use crate::utils::{emit_to_users, now_ts, recipients_for_source};

#[derive(Debug, Clone, Serialize)]
pub struct SendMessageResult {
    pub id: i64,
    pub sender_user_id: u64,
    pub source: MessageSource,
    pub content: Vec<MessageSegment>,
    pub quote_message_id: Option<i64>,
    pub recall: MessageRecallInfo,
    pub created_at: u64,
}

#[derive(Clone)]
pub struct MessageService {
    repo: MessageRepo,
    group_repo: GroupRepo,
}

impl MessageService {
    pub fn new(repo: MessageRepo, group_repo: GroupRepo) -> Self {
        Self { repo, group_repo }
    }

    pub async fn send(
        &self,
        core: &CoreContainer,
        user_id: u64,
        source: MessageSource,
        content: Vec<MessageSegment>,
        quote_message_id: Option<i64>,
    ) -> AppResult<SendMessageResult> {
        core.require_user_context(user_id)?;

        let now = now_ts();

        if let MessageSource::Group { group_id } = &source {
            let member = self
                .group_repo
                .get_group_member(*group_id, user_id)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("user {} is not in group {}", user_id, group_id))
                })?;

            if let Some(mute_until) = member.mute_until
                && mute_until > now
            {
                return Err(AppError::validation(format!(
                    "user {} is muted in group {} until {}",
                    user_id, group_id, mute_until
                )));
            }

            let whole_mute = self.group_repo.get_group_whole_mute(*group_id).await?;

            if let Some(state) = whole_mute {
                let active = state.muted
                    && state
                        .mute_until
                        .map(|mute_until| mute_until > now)
                        .unwrap_or(true);
                if active && matches!(member.role, GroupRole::Member) {
                    return Err(AppError::validation(format!(
                        "group {} is in whole-group mute mode",
                        group_id
                    )));
                }
            }

            self.group_repo
                .update_group_member_last_sent_at(*group_id, user_id, now)
                .await?;
        }

        if let Some(quoted_id) = quote_message_id {
            let quoted_message =
                self.repo
                    .get_message_by_id(quoted_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::validation(format!("quoted message {} not found", quoted_id))
                    })?;

            if !message_in_same_session(&quoted_message, user_id, &source) {
                return Err(AppError::validation(
                    "quoted message is not in current conversation",
                ));
            }
        }

        let content_json = serde_json::to_string(&content)?;

        let (source_type, source_id) = source.to_db_parts();

        let saved = self
            .repo
            .insert_message(NewMessageRecord {
                sender_user_id: user_id,
                source_type: source_type.to_string(),
                source_id,
                content_json,
                quote_message_id,
                created_at: now,
            })
            .await?;

        let event = InternalEvent::Message {
            sender: user_id,
            group_id: match source {
                MessageSource::Group { group_id } => Some(group_id),
                _ => None,
            },
            content: content.clone(),
            time: now,
        };
        let recipients =
            recipients_for_source(core, &self.group_repo, &source, user_id, Some(user_id)).await;
        emit_to_users(core, recipients, event);

        Ok(SendMessageResult {
            id: saved.id,
            sender_user_id: user_id,
            source,
            content,
            quote_message_id,
            recall: MessageRecallInfo {
                recalled: false,
                recalled_by_user_id: None,
            },
            created_at: now,
        })
    }

    pub async fn list_history(
        &self,
        user_id: u64,
        source: MessageSource,
        limit: usize,
    ) -> AppResult<Vec<SendMessageResult>> {
        let limit_i64 =
            i64::try_from(limit).map_err(|_| AppError::validation("limit is too large"))?;
        let (source_type, source_id) = source.to_db_parts();
        let rows = self
            .repo
            .list_messages(user_id, source_type, source_id, limit_i64)
            .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn recall_message(
        &self,
        core: &CoreContainer,
        user_id: u64,
        message_id: i64,
    ) -> AppResult<MessageEntity> {
        core.require_user_context(user_id)?;

        let current = self
            .repo
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("message {} not found", message_id)))?;

        if current.is_recalled {
            return Err(AppError::conflict("message already recalled"));
        }

        let is_sender = current.sender_user_id == user_id;
        if !is_sender {
            match current.source_type.as_str() {
                "group" => {
                    let group_id = current.source_id;
                    let operator_member = self
                        .group_repo
                        .get_group_member(group_id, user_id)
                        .await?
                        .ok_or_else(|| AppError::validation("operator is not in group"))?;
                    if matches!(operator_member.role, GroupRole::Member) {
                        return Err(AppError::validation(
                            "only sender/owner/admin can recall group messages",
                        ));
                    }
                }
                _ => {
                    return Err(AppError::validation(
                        "only sender can recall private messages",
                    ));
                }
            }
        }

        let row = self
            .repo
            .mark_message_recalled(message_id, user_id)
            .await?
            .ok_or_else(|| AppError::conflict("message already recalled"))?;

        let entity: MessageEntity = row.try_into()?;
        let recipients = recipients_for_source(
            core,
            &self.group_repo,
            &entity.source,
            user_id,
            Some(entity.sender_user_id),
        )
        .await;
        emit_to_users(
            core,
            recipients,
            InternalEvent::MessageRecalled {
                message_id: entity.message_id,
                source: entity.source.clone(),
                recalled_by_user_id: user_id,
                time: now_ts(),
            },
        );

        Ok(entity)
    }
}

impl TryFrom<MessageRecord> for SendMessageResult {
    type Error = AppError;

    fn try_from(row: MessageRecord) -> Result<Self, Self::Error> {
        let source: MessageSource = (row.source_type.as_str(), row.source_id)
            .try_into()
            .map_err(AppError::internal)?;
        let content: Vec<MessageSegment> = serde_json::from_str(&row.content_json)?;

        Ok(Self {
            id: row.id,
            sender_user_id: row.sender_user_id,
            source,
            content,
            quote_message_id: row.quote_message_id,
            recall: MessageRecallInfo {
                recalled: row.is_recalled,
                recalled_by_user_id: row.recalled_by_user_id,
            },
            created_at: row.created_at,
        })
    }
}

impl TryFrom<MessageRecord> for MessageEntity {
    type Error = AppError;

    fn try_from(row: MessageRecord) -> Result<Self, Self::Error> {
        let source: MessageSource = (row.source_type.as_str(), row.source_id)
            .try_into()
            .map_err(AppError::internal)?;
        let content: Vec<MessageSegment> = serde_json::from_str(&row.content_json)?;

        Ok(Self {
            message_id: row.id,
            sender_user_id: row.sender_user_id,
            source,
            content,
            quote_message_id: row.quote_message_id,
            created_at: row.created_at,
            recall: MessageRecallInfo {
                recalled: row.is_recalled,
                recalled_by_user_id: row.recalled_by_user_id,
            },
        })
    }
}

fn message_in_same_session(message: &MessageRecord, user_id: u64, source: &MessageSource) -> bool {
    match source {
        MessageSource::Private { peer_user_id } => {
            if message.source_type != "private" {
                return false;
            }

            (message.sender_user_id == user_id && message.source_id == *peer_user_id)
                || (message.sender_user_id == *peer_user_id && message.source_id == user_id)
        }
        MessageSource::Group { group_id } => {
            message.source_type == "group" && message.source_id == *group_id
        }
    }
}
