use std::collections::HashSet;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::{
    GroupEventPayload, GroupMemberProfile, GroupProfile, GroupRole, GroupWholeMuteState,
    InternalEvent,
};
use crate::utils::{emit_to_group_members, now_ts};

use super::{GroupService, MuteGroupMemberResult};

impl GroupService {
    pub async fn upsert_group(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        group_name: String,
        max_member_count: u32,
        initial_member_user_ids: Vec<u64>,
    ) -> AppResult<GroupProfile> {
        if group_id == 0 {
            return Err(AppError::validation(
                "group_id must be specified and greater than 0",
            ));
        }

        core.require_user_context(user_id)?;

        if initial_member_user_ids.contains(&user_id) {
            return Err(AppError::validation(
                "initial_member_user_ids must not contain owner user_id",
            ));
        }

        let group = GroupProfile {
            group_id,
            group_name,
            owner_user_id: user_id,
            member_count: 0,
            max_member_count,
        };

        self.repo.upsert_group(&group).await?;

        let owner_member = GroupMemberProfile {
            group_id,
            user_id,
            card: String::new(),
            title: String::new(),
            role: GroupRole::Owner,
            joined_at: now_ts(),
            last_sent_at: 0,
            mute_until: None,
        };
        self.repo.upsert_group_member(&owner_member).await?;

        let mut dedup_member_ids = HashSet::new();
        for target_user_id in initial_member_user_ids {
            if dedup_member_ids.insert(target_user_id) {
                let member = GroupMemberProfile {
                    group_id,
                    user_id: target_user_id,
                    card: String::new(),
                    title: String::new(),
                    role: GroupRole::Member,
                    joined_at: now_ts(),
                    last_sent_at: 0,
                    mute_until: None,
                };
                self.repo.upsert_group_member(&member).await?;
            }
        }

        Ok(group)
    }

    pub async fn list_groups(&self) -> AppResult<Vec<GroupProfile>> {
        self.repo.list_groups().await.map_err(Into::into)
    }

    pub async fn list_user_groups(
        &self,
        core: &CoreContainer,
        user_id: u64,
    ) -> AppResult<Vec<GroupProfile>> {
        core.require_user_context(user_id)?;
        self.repo
            .list_user_groups(user_id)
            .await
            .map_err(Into::into)
    }

    pub async fn ensure_group_member(
        &self,
        group_id: u64,
        user_id: u64,
    ) -> AppResult<GroupMemberProfile> {
        let member = self
            .repo
            .get_group_member(group_id, user_id)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("user {} is not in group {}", user_id, group_id))
            })?;
        Ok(member)
    }

    pub async fn upsert_group_member(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        target_user_id: u64,
    ) -> AppResult<GroupMemberProfile> {
        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;

        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can add group members",
            ));
        }

        if let Some(member) = self.repo.get_group_member(group_id, target_user_id).await? {
            return Ok(member);
        }

        let event_time = now_ts();

        let member = GroupMemberProfile {
            group_id,
            user_id: target_user_id,
            card: String::new(),
            title: String::new(),
            role: GroupRole::Member,
            joined_at: event_time,
            last_sent_at: 0,
            mute_until: None,
        };

        self.repo.upsert_group_member(&member).await?;

        self.save_group_event(
            group_id,
            GroupEventPayload::MemberJoined {
                operator_user_id: user_id,
                joined_user_id: target_user_id,
            },
            event_time,
        )
        .await?;

        emit_to_group_members(
            core,
            &self.repo,
            group_id,
            InternalEvent::GroupMemberJoined {
                group_id,
                operator_user_id: user_id,
                target_user_id,
                time: event_time,
            },
        )
        .await;

        Ok(member)
    }

    pub async fn list_group_members(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Vec<GroupMemberProfile>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .list_group_members(group_id)
            .await
            .map_err(Into::into)
    }

    pub async fn mute_group_member(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        target_user_id: u64,
        duration_seconds: u64,
    ) -> AppResult<MuteGroupMemberResult> {
        if user_id == target_user_id {
            return Err(AppError::validation("cannot mute yourself"));
        }

        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;
        let target = self.ensure_group_member(group_id, target_user_id).await?;

        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can mute group members",
            ));
        }

        if matches!(target.role, GroupRole::Owner) {
            return Err(AppError::validation("owner cannot be muted"));
        }

        if matches!(operator.role, GroupRole::Admin) && matches!(target.role, GroupRole::Admin) {
            return Err(AppError::validation("admin cannot mute another admin"));
        }

        let mute_until = if duration_seconds == 0 {
            None
        } else {
            Some(now_ts() + duration_seconds)
        };

        let updated = self
            .repo
            .set_group_member_mute(group_id, target_user_id, mute_until)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "target user {} is not in group {}",
                    target_user_id, group_id
                ))
            })?;

        let result = MuteGroupMemberResult {
            group_id,
            target_user_id,
            muted: updated.mute_until.is_some(),
            mute_until: updated.mute_until,
        };

        let event_time = now_ts();

        self.save_group_event(
            result.group_id,
            GroupEventPayload::MemberMuted {
                operator_user_id: user_id,
                target_user_id: result.target_user_id,
                mute_until: result.mute_until,
            },
            event_time,
        )
        .await?;

        let event = InternalEvent::GroupMemberMuted {
            group_id: result.group_id,
            operator_user_id: user_id,
            target_user_id: result.target_user_id,
            mute_until: result.mute_until,
            time: event_time,
        };
        emit_to_group_members(core, &self.repo, group_id, event).await;

        Ok(result)
    }

    pub async fn set_group_whole_mute(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        duration_seconds: u64,
    ) -> AppResult<GroupWholeMuteState> {
        core.require_user_context(user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;

        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation(
                "only owner/admin can set whole-group mute",
            ));
        }

        let muted = duration_seconds > 0;
        let mute_until = if muted {
            Some(now_ts() + duration_seconds)
        } else {
            None
        };

        let state = self
            .repo
            .set_group_whole_mute(group_id, muted, mute_until, user_id, now_ts())
            .await?;

        let event = InternalEvent::GroupWholeMuteUpdated {
            group_id: state.group_id,
            operator_user_id: user_id,
            muted: state.muted,
            mute_until: state.mute_until,
            time: state.updated_at,
        };
        emit_to_group_members(core, &self.repo, group_id, event).await;

        Ok(state)
    }

    pub async fn get_group_whole_mute(
        &self,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<Option<GroupWholeMuteState>> {
        self.ensure_group_member(group_id, user_id).await?;
        self.repo
            .get_group_whole_mute(group_id)
            .await
            .map_err(Into::into)
    }
}
