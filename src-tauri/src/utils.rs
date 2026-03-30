use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::CoreContainer;
use crate::models::{InternalEvent, MessageSource};
use crate::persistence::GroupRepo;

pub fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn emit_to_users(
    core: &CoreContainer,
    user_ids: impl IntoIterator<Item = u64>,
    event: InternalEvent,
) {
    for user_id in user_ids {
        if let Ok(ctx) = core.require_user_context(user_id)
            && let Err(err) = ctx.event_tx.send(event.clone())
        {
            eprintln!("failed to emit event to user {user_id}: {err}");
        }
    }
}

pub async fn emit_to_group_members(
    core: &CoreContainer,
    group_repo: &GroupRepo,
    group_id: u64,
    event: InternalEvent,
) {
    match group_repo.list_group_members(group_id).await {
        Ok(members) => {
            emit_to_users(core, members.iter().map(|member| member.user_id), event);
        }
        Err(err) => {
            eprintln!("failed to list group members for group {group_id}: {err}");
        }
    }
}

pub async fn recipients_for_source(
    core: &CoreContainer,
    group_repo: &GroupRepo,
    source: &MessageSource,
    actor_user_id: u64,
    other_user_id: Option<u64>,
) -> HashSet<u64> {
    let mut recipients = HashSet::new();
    recipients.insert(actor_user_id);
    if let Some(other_user_id) = other_user_id {
        recipients.insert(other_user_id);
    }

    match source {
        MessageSource::Private { peer_user_id } => {
            recipients.insert(*peer_user_id);
        }
        MessageSource::Group { group_id } => match group_repo.list_group_members(*group_id).await {
            Ok(members) => {
                for member in members {
                    recipients.insert(member.user_id);
                }
            }
            Err(err) => {
                eprintln!(
                    "failed to list group members for recipient calc group {group_id}: {err}"
                );
            }
        },
    }

    recipients.retain(|user_id| core.user_context(*user_id).is_some());
    recipients
}
