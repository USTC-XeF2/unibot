use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::models::internal::NoticeType;
use crate::models::{GroupMemberProfile, GroupProfile, GroupRole, InternalEvent};
use crate::utils::{emit_to_group_members, emit_to_users, now_ts};

use super::GroupService;

impl GroupService {
    pub async fn kick_group_member(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        target_user_id: u64,
    ) -> AppResult<()> {
        if user_id == target_user_id {
            return Err(AppError::validation("cannot kick yourself"));
        }

        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;
        let target = self.ensure_group_member(group_id, target_user_id).await?;

        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation("only owner/admin can kick members"));
        }
        if matches!(target.role, GroupRole::Owner) {
            return Err(AppError::validation("owner cannot be kicked"));
        }
        if matches!(operator.role, GroupRole::Admin) && matches!(target.role, GroupRole::Admin) {
            return Err(AppError::validation("admin cannot kick another admin"));
        }

        self.repo
            .remove_group_member(group_id, target_user_id)
            .await?;

        let event = InternalEvent::Notice {
            group_id,
            actor: user_id,
            target: target_user_id,
            notice_type: NoticeType::Kick,
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, group_id, event.clone()).await;
        emit_to_users(core, [target_user_id], event);

        Ok(())
    }

    pub async fn set_group_member_role(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        target_user_id: u64,
        is_admin: bool,
    ) -> AppResult<GroupMemberProfile> {
        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;
        if !matches!(operator.role, GroupRole::Owner) {
            return Err(AppError::validation("only owner can change admin role"));
        }

        let target = self.ensure_group_member(group_id, target_user_id).await?;

        if matches!(target.role, GroupRole::Owner) {
            return Err(AppError::validation("cannot change owner role"));
        }

        let role = if is_admin {
            GroupRole::Admin
        } else {
            GroupRole::Member
        };

        let updated = self
            .repo
            .update_group_member_role(group_id, target_user_id, role)
            .await?
            .ok_or_else(|| AppError::validation("target user is not in group"))?;

        let event = InternalEvent::Notice {
            group_id,
            actor: user_id,
            target: target_user_id,
            notice_type: NoticeType::AdminChange,
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, group_id, event).await;

        Ok(updated)
    }

    pub async fn set_group_member_title(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        target_user_id: u64,
        title: String,
    ) -> AppResult<GroupMemberProfile> {
        core.require_user_context(user_id)?;
        core.require_user_context(target_user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;
        if !matches!(operator.role, GroupRole::Owner) {
            return Err(AppError::validation(
                "only owner can set group member title",
            ));
        }

        let updated = self
            .repo
            .update_group_member_title(group_id, target_user_id, title)
            .await?
            .ok_or_else(|| AppError::validation("target user is not in group"))?;

        let event = InternalEvent::GroupMemberTitleUpdated {
            group_id,
            operator_user_id: user_id,
            target_user_id,
            time: now_ts(),
        };
        emit_to_group_members(core, &self.repo, group_id, event).await;

        Ok(updated)
    }

    pub async fn rename_group(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
        group_name: String,
    ) -> AppResult<GroupProfile> {
        core.require_user_context(user_id)?;

        let operator = self.ensure_group_member(group_id, user_id).await?;
        if matches!(operator.role, GroupRole::Member) {
            return Err(AppError::validation("only owner/admin can rename group"));
        }

        let name = group_name.trim();
        if name.is_empty() {
            return Err(AppError::validation("group_name cannot be empty"));
        }

        self.repo
            .update_group_name(group_id, name.to_string())
            .await?
            .ok_or_else(|| AppError::not_found(format!("group {} not found", group_id)))
    }

    pub async fn leave_group(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<()> {
        core.require_user_context(user_id)?;

        let member = self.ensure_group_member(group_id, user_id).await?;

        if matches!(member.role, GroupRole::Owner) {
            return Err(AppError::validation(
                "owner cannot leave group directly, dissolve instead",
            ));
        }

        self.repo.remove_group_member(group_id, user_id).await?;

        Ok(())
    }

    pub async fn dissolve_group(
        &self,
        core: &CoreContainer,
        user_id: u64,
        group_id: u64,
    ) -> AppResult<()> {
        core.require_user_context(user_id)?;

        let group = self
            .repo
            .get_group(group_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("group {} not found", group_id)))?;

        if group.owner_user_id != user_id {
            return Err(AppError::validation("only owner can dissolve group"));
        }

        let deleted = self.repo.delete_group(group_id).await?;
        if !deleted {
            return Err(AppError::not_found(format!("group {} not found", group_id)));
        }

        Ok(())
    }
}
