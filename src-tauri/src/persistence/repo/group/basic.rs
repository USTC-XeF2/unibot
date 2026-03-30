use crate::models::{GroupMemberProfile, GroupProfile, GroupRole, GroupWholeMuteState};

use super::GroupRepo;
use super::types::{GroupMemberRow, GroupRow, GroupWholeMuteRow};

impl GroupRepo {
    pub async fn upsert_group(&self, group: &GroupProfile) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO groups (group_id, group_name, owner_user_id)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(group_id) DO UPDATE SET
                group_name = excluded.group_name,
                owner_user_id = excluded.owner_user_id
            "#,
        )
        .bind(group.group_id as i64)
        .bind(&group.group_name)
        .bind(group.owner_user_id as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_groups(&self) -> Result<Vec<GroupProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT
                g.group_id,
                g.group_name,
                g.owner_user_id,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = g.group_id
                ) AS member_count
            FROM groups g
            ORDER BY g.group_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(GroupProfile::from).collect())
    }

    pub async fn list_user_groups(&self, user_id: u64) -> Result<Vec<GroupProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT
                g.group_id,
                g.group_name,
                g.owner_user_id,
                (
                    SELECT COUNT(*)
                    FROM group_members gm_count
                    WHERE gm_count.group_id = g.group_id
                ) AS member_count
            FROM groups g
            INNER JOIN group_members gm
                ON gm.group_id = g.group_id
            WHERE gm.user_id = ?1
            ORDER BY g.group_id ASC
            "#,
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(GroupProfile::from).collect())
    }

    pub async fn get_group(&self, group_id: u64) -> Result<Option<GroupProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT
                g.group_id,
                g.group_name,
                g.owner_user_id,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = g.group_id
                ) AS member_count
            FROM groups g
            WHERE g.group_id = ?1
            "#,
        )
        .bind(group_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupProfile::from))
    }

    pub async fn upsert_group_member(
        &self,
        member: &GroupMemberProfile,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_members (
                group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(group_id, user_id) DO UPDATE SET
                card = excluded.card,
                title = excluded.title,
                role = excluded.role,
                joined_at = excluded.joined_at,
                last_sent_at = excluded.last_sent_at,
                mute_until = excluded.mute_until
            "#,
        )
        .bind(member.group_id as i64)
        .bind(member.user_id as i64)
        .bind(&member.card)
        .bind(&member.title)
        .bind(member.role)
        .bind(member.joined_at as i64)
        .bind(member.last_sent_at as i64)
        .bind(member.mute_until.map(|ts| ts as i64))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_group_members(
        &self,
        group_id: u64,
    ) -> Result<Vec<GroupMemberProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            SELECT group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
            FROM group_members
            WHERE group_id = ?1
            ORDER BY user_id ASC
            "#,
        )
        .bind(group_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(GroupMemberProfile::from).collect())
    }

    pub async fn get_group_member(
        &self,
        group_id: u64,
        user_id: u64,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            SELECT group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
            FROM group_members
            WHERE group_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(group_id as i64)
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupMemberProfile::from))
    }

    pub async fn set_group_member_mute(
        &self,
        group_id: u64,
        user_id: u64,
        mute_until: Option<u64>,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            UPDATE group_members
            SET mute_until = ?3
            WHERE group_id = ?1 AND user_id = ?2
            RETURNING group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
            "#,
        )
        .bind(group_id as i64)
        .bind(user_id as i64)
        .bind(mute_until.map(|t| t as i64))
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupMemberProfile::from))
    }

    pub async fn remove_group_member(
        &self,
        group_id: u64,
        user_id: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM group_members
            WHERE group_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(group_id as i64)
        .bind(user_id as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_group_member_role(
        &self,
        group_id: u64,
        user_id: u64,
        role: GroupRole,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            UPDATE group_members
            SET role = ?3
            WHERE group_id = ?1 AND user_id = ?2
            RETURNING group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
            "#,
        )
        .bind(group_id as i64)
        .bind(user_id as i64)
        .bind(role)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupMemberProfile::from))
    }

    pub async fn update_group_member_title(
        &self,
        group_id: u64,
        user_id: u64,
        title: String,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            UPDATE group_members
            SET title = ?3
            WHERE group_id = ?1 AND user_id = ?2
            RETURNING group_id, user_id, card, title, role, joined_at, last_sent_at, mute_until
            "#,
        )
        .bind(group_id as i64)
        .bind(user_id as i64)
        .bind(title)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupMemberProfile::from))
    }

    pub async fn update_group_name(
        &self,
        group_id: u64,
        group_name: String,
    ) -> Result<Option<GroupProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRow>(
            r#"
            UPDATE groups
            SET group_name = ?2
            WHERE group_id = ?1
            RETURNING
                group_id,
                group_name,
                owner_user_id,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = groups.group_id
                ) AS member_count
            "#,
        )
        .bind(group_id as i64)
        .bind(group_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupProfile::from))
    }

    pub async fn delete_group(&self, group_id: u64) -> Result<bool, sqlx::Error> {
        let deleted = sqlx::query(
            r#"
            DELETE FROM groups
            WHERE group_id = ?1
            "#,
        )
        .bind(group_id as i64)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0;

        Ok(deleted)
    }

    pub async fn update_group_member_last_sent_at(
        &self,
        group_id: u64,
        user_id: u64,
        last_sent_at: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE group_members
            SET last_sent_at = ?3
            WHERE group_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(group_id as i64)
        .bind(user_id as i64)
        .bind(last_sent_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_group_whole_mute(
        &self,
        group_id: u64,
        muted: bool,
        mute_until: Option<u64>,
        operator_user_id: u64,
        updated_at: u64,
    ) -> Result<GroupWholeMuteState, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupWholeMuteRow>(
            r#"
            INSERT INTO group_whole_mute (group_id, muted, mute_until, operator_user_id, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(group_id) DO UPDATE SET
                muted = excluded.muted,
                mute_until = excluded.mute_until,
                operator_user_id = excluded.operator_user_id,
                updated_at = excluded.updated_at
            RETURNING group_id, muted, mute_until, operator_user_id, updated_at
            "#,
        )
        .bind(group_id as i64)
        .bind(muted)
        .bind(mute_until.map(|t| t as i64))
        .bind(operator_user_id as i64)
        .bind(updated_at as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(GroupWholeMuteState::from(row))
    }

    pub async fn get_group_whole_mute(
        &self,
        group_id: u64,
    ) -> Result<Option<GroupWholeMuteState>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupWholeMuteRow>(
            r#"
            SELECT group_id, muted, mute_until, operator_user_id, updated_at
            FROM group_whole_mute
            WHERE group_id = ?1
            "#,
        )
        .bind(group_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(GroupWholeMuteState::from))
    }
}
