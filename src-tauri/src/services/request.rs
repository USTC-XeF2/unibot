use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{FriendRequestEntity, InternalEvent, RequestState};
use crate::persistence::{NewFriendRequestRecord, UserRepo};
use crate::utils::{emit_to_users, now_ts};

#[derive(Clone)]
pub struct RequestService {
    user_repo: UserRepo,
}

impl RequestService {
    pub fn new(user_repo: UserRepo) -> Self {
        Self { user_repo }
    }

    pub async fn create_friend_request(
        &self,
        core: &CoreContainer,
        user_id: u64,
        target_user_id: u64,
        comment: String,
    ) -> AppResult<FriendRequestEntity> {
        if user_id == target_user_id {
            return Err(AppError::validation(
                "cannot send friend request to yourself",
            ));
        }

        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let already_friends = self.user_repo.are_friends(user_id, target_user_id).await?;
        if already_friends {
            return Err(AppError::conflict("users are already friends"));
        }

        let has_pending = self
            .user_repo
            .has_pending_friend_request_between(user_id, target_user_id)
            .await?;
        if has_pending {
            return Err(AppError::conflict(
                "a pending friend request already exists between the users",
            ));
        }

        let created = self
            .user_repo
            .create_friend_request(NewFriendRequestRecord {
                initiator_user_id: user_id,
                target_user_id,
                comment,
                created_at: now_ts(),
            })
            .await?;

        emit_to_users(
            core,
            [created.initiator_user_id, created.target_user_id],
            InternalEvent::FriendRequestCreated {
                request_id: created.request_id,
                initiator_user_id: created.initiator_user_id,
                target_user_id: created.target_user_id,
                time: created.created_at,
            },
        );

        Ok(created)
    }

    pub async fn list_friend_requests(&self, user_id: u64) -> AppResult<Vec<FriendRequestEntity>> {
        self.user_repo
            .list_friend_requests(user_id)
            .await
            .map_err(Into::into)
    }

    pub async fn handle_friend_request(
        &self,
        core: &CoreContainer,
        user_id: u64,
        request_id: i64,
        state: RequestState,
    ) -> AppResult<FriendRequestEntity> {
        core.require_user_context(user_id)?;

        let updated = self
            .user_repo
            .handle_friend_request_for_target(request_id, state, user_id, user_id, now_ts())
            .await?;

        let updated = if let Some(updated) = updated {
            updated
        } else {
            let current = self.user_repo.get_friend_request_by_id(request_id).await?;
            match current {
                None => {
                    return Err(AppError::not_found(format!(
                        "friend request {} not found",
                        request_id
                    )));
                }
                Some(entity) if entity.state != RequestState::Pending => {
                    return Err(AppError::conflict(
                        "friend request has already been handled",
                    ));
                }
                Some(_) => {
                    return Err(AppError::validation(
                        "only target user can handle friend request",
                    ));
                }
            }
        };

        emit_to_users(
            core,
            [updated.initiator_user_id, updated.target_user_id],
            InternalEvent::FriendRequestHandled {
                request_id: updated.request_id,
                initiator_user_id: updated.initiator_user_id,
                target_user_id: updated.target_user_id,
                operator_user_id: user_id,
                state,
                time: now_ts(),
            },
        );

        Ok(updated)
    }

    pub async fn delete_friend(
        &self,
        core: &CoreContainer,
        user_id: u64,
        friend_user_id: u64,
    ) -> AppResult<()> {
        if user_id == friend_user_id {
            return Err(AppError::validation("cannot delete yourself from friends"));
        }

        core.require_user_context(user_id)?;
        core.require_user_context(friend_user_id)?;

        let deleted = self
            .user_repo
            .remove_friendship_pair(user_id, friend_user_id)
            .await?;
        if !deleted {
            return Err(AppError::not_found("users are not friends"));
        }

        Ok(())
    }
}
