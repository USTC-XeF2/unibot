use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupEventPayload, GroupRequestEntity, GroupRequestType, GroupRole, InternalEvent, RequestState,
};
use crate::persistence::NewGroupRequestRecord;
use crate::utils::{emit_to_group_members, emit_to_users, now_ts};

use super::GroupService;

impl GroupService {
    pub async fn create_group_request(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        request_type: GroupRequestType,
        target_user_id: Option<u64>,
        comment: Option<String>,
    ) -> AppResult<GroupRequestEntity> {
        self.repo
            .get_group(group_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("group {} not found", group_id)))?;

        core.require_user_context(user_id)?;

        if self
            .repo
            .has_pending_group_request(group_id, request_type, user_id, target_user_id)
            .await?
        {
            return Err(AppError::conflict("a pending group request already exists"));
        }

        match request_type {
            GroupRequestType::Join => {
                if target_user_id.is_some() {
                    return Err(AppError::validation(
                        "join request must not specify target_user_id",
                    ));
                }

                let already_member = self
                    .repo
                    .get_group_member(group_id, user_id)
                    .await?
                    .is_some();
                if already_member {
                    return Err(AppError::conflict("user is already a group member"));
                }
            }
            GroupRequestType::Invite => {
                let target_user_id = target_user_id.ok_or_else(|| {
                    AppError::validation("invite request requires target_user_id")
                })?;

                core.require_user_context(target_user_id)?;

                let initiator_member = self.ensure_group_member(group_id, user_id).await?;

                if matches!(initiator_member.role, GroupRole::Member) {
                    return Err(AppError::validation("only owner/admin can invite users"));
                }

                let target_member = self.repo.get_group_member(group_id, target_user_id).await?;
                if target_member.is_some() {
                    return Err(AppError::conflict("target user is already in group"));
                }
            }
        }

        let created = self
            .repo
            .create_group_request(NewGroupRequestRecord {
                group_id,
                request_type,
                initiator_user_id: user_id,
                target_user_id,
                comment,
                created_at: now_ts(),
            })
            .await?;

        let event = InternalEvent::GroupRequestCreated {
            request_id: created.request_id,
            group_id: created.group_id,
            request_type: created.request_type,
            initiator_user_id: created.initiator_user_id,
            target_user_id: created.target_user_id,
            time: created.created_at,
        };

        match created.request_type {
            GroupRequestType::Join => {
                emit_to_group_members(core, &self.repo, created.group_id, event.clone()).await;
                emit_to_users(core, [user_id], event);
            }
            GroupRequestType::Invite => {
                let mut recipients = vec![created.initiator_user_id];
                if let Some(target_user_id) = created.target_user_id {
                    recipients.push(target_user_id);
                }
                emit_to_users(core, recipients, event);
            }
        }

        Ok(created)
    }

    pub async fn list_group_requests(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Vec<GroupRequestEntity>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .list_group_requests(group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn handle_group_request(
        &self,
        core: &CoreContainer,
        user_id: u64,
        request_id: i64,
        state: RequestState,
    ) -> AppResult<GroupRequestEntity> {
        core.require_user_context(user_id)?;

        let current = self
            .repo
            .get_group_request_by_id(request_id)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("group request {} not found", request_id))
            })?;

        if current.state != RequestState::Pending {
            return Err(AppError::conflict("group request has already been handled"));
        }

        match current.request_type {
            GroupRequestType::Join => {
                let operator_member = self.ensure_group_member(current.group_id, user_id).await?;
                if matches!(operator_member.role, GroupRole::Member) {
                    return Err(AppError::validation(
                        "only owner/admin can handle join requests",
                    ));
                }
            }
            GroupRequestType::Invite => {
                let target_user_id = current
                    .target_user_id
                    .ok_or_else(|| AppError::internal("invite request is missing target user"))?;

                let is_target_operator = user_id == target_user_id;
                let admin_or_owner = self
                    .repo
                    .get_group_member(current.group_id, user_id)
                    .await?
                    .map(|m| !matches!(m.role, GroupRole::Member))
                    .unwrap_or(false);

                if !is_target_operator && !admin_or_owner {
                    return Err(AppError::validation(
                        "invite request can only be handled by target user or owner/admin",
                    ));
                }
            }
        }

        let handled = self
            .repo
            .handle_group_request(request_id, state, user_id, now_ts(), now_ts())
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("group request {} not found", request_id))
            })?;

        let event = InternalEvent::GroupRequestHandled {
            request_id: handled.request_id,
            group_id: handled.group_id,
            request_type: handled.request_type,
            initiator_user_id: handled.initiator_user_id,
            target_user_id: handled.target_user_id,
            operator_user_id: user_id,
            state,
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, handled.group_id, event.clone()).await;
        emit_to_users(core, [handled.initiator_user_id], event.clone());
        if let Some(target_user_id) = handled.target_user_id {
            emit_to_users(core, [target_user_id], event);
        }

        if state == RequestState::Accepted {
            let joined_user_id = match handled.request_type {
                GroupRequestType::Join => handled.initiator_user_id,
                GroupRequestType::Invite => handled.target_user_id.unwrap_or(0),
            };
            let event_time = handled.handled_at.unwrap_or_else(now_ts);

            if joined_user_id != 0 {
                self.save_group_event(
                    handled.group_id,
                    GroupEventPayload::MemberJoined {
                        operator_user_id: user_id,
                        joined_user_id,
                    },
                    event_time,
                )
                .await?;

                emit_to_group_members(
                    core,
                    &self.repo,
                    handled.group_id,
                    InternalEvent::GroupMemberJoined {
                        group_id: handled.group_id,
                        operator_user_id: user_id,
                        target_user_id: joined_user_id,
                        time: event_time,
                    },
                )
                .await;
            }
        }

        Ok(handled)
    }
}
