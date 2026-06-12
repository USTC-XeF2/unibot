use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{DevToolsEvent, InternalEvent, MessageSource};
use crate::persistence::GroupRepo;

pub fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn new_db_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

pub fn emit_to_users<I, S>(core: &CoreContainer, user_ids: I, event: InternalEvent)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for user_id in user_ids {
        if let Ok(ctx) = core.require_user_context(user_id.as_ref()) {
            let _ = ctx.event_tx.send(event.clone());
        }
    }

    // Emit a single copy to the devtools firehose. The event stream panel
    // shows what the system produced, not per-recipient delivery logs.
    core.send_devtools_event(DevToolsEvent { event });
}

pub async fn emit_to_group_members(
    core: &CoreContainer,
    group_repo: &GroupRepo,
    group_id: &str,
    event: InternalEvent,
) -> AppResult<()> {
    let members = group_repo.list_group_members(group_id).await.map_err(|e| {
        AppError::storage(format!("failed to list group members for {group_id}: {e}"))
    })?;
    emit_to_users(core, members.iter().map(|member| &member.user_id), event);
    Ok(())
}

pub async fn recipients_for_source(
    core: &CoreContainer,
    group_repo: &GroupRepo,
    source: &MessageSource,
    actor_user_id: &str,
    other_user_id: Option<&str>,
) -> HashSet<String> {
    let mut recipients = HashSet::new();
    recipients.insert(actor_user_id.to_string());
    if let Some(other_user_id) = other_user_id {
        recipients.insert(other_user_id.to_string());
    }

    match source {
        MessageSource::Private { peer_user_id } => {
            recipients.insert(peer_user_id.clone());
        }
        MessageSource::Group { group_id } => match group_repo.list_group_members(group_id).await {
            Ok(members) => {
                for member in members {
                    recipients.insert(member.user_id);
                }
            }
            Err(err) => {
                tracing::warn!(
                    target: "utils",
                    "failed to list group members for recipient calc group {group_id}: {err}"
                );
            }
        },
    }

    recipients.retain(|user_id| core.user_context(user_id).is_some());
    recipients
}
