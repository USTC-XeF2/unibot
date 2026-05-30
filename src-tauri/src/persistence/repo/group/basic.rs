use crate::models::{GroupMemberProfile, GroupProfile, GroupRole, GroupWholeMuteState};
use crate::persistence::repo::codecs;

use super::GroupRepo;
use super::types::{GroupMemberRow, GroupRow, GroupWholeMuteRow};

impl GroupRepo {
    pub async fn upsert_group(&self, group: &GroupProfile) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO chat_groups (
                group_id, group_name, group_owner_user_id, group_source, max_member_count,
                group_status, dissolved_at, unavailable_at
            )
            VALUES (?1, ?2, ?3, 'simulated', ?4, 'active', NULL, NULL)
            ON CONFLICT(group_id) DO UPDATE SET
                group_name = excluded.group_name,
                group_owner_user_id = excluded.group_owner_user_id,
                max_member_count = excluded.max_member_count,
                group_status = 'active',
                dissolved_at = NULL,
                unavailable_at = NULL,
                updated_at = unixepoch() * 1000
            "#,
        )
        .bind(&group.group_id)
        .bind(&group.group_name)
        .bind(&group.owner_user_id)
        .bind(group.max_member_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_groups(&self) -> Result<Vec<GroupProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT group_id,
                   group_name,
                   group_owner_user_id AS owner_user_id,
                   member_count,
                   max_member_count,
                   group_status
            FROM chat_groups
            WHERE group_status != 'dissolved'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn list_user_groups(&self, user_id: &str) -> Result<Vec<GroupProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT g.group_id,
                   g.group_name,
                   g.group_owner_user_id AS owner_user_id,
                   g.member_count,
                   g.max_member_count,
                   g.group_status
            FROM user_groups ug
            JOIN chat_groups g ON g.group_id = ug.group_id
            WHERE ug.owner_user_id = ?1
              AND g.group_status != 'dissolved'
            ORDER BY ug.sort_order ASC, g.created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_group(&self, group_id: &str) -> Result<Option<GroupProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT group_id,
                   group_name,
                   group_owner_user_id AS owner_user_id,
                   member_count,
                   max_member_count,
                   group_status
            FROM chat_groups
            WHERE group_id = ?1
            "#,
        )
        .bind(group_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn upsert_group_member(
        &self,
        member: &GroupMemberProfile,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO group_members (
                group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(group_id, user_id) DO UPDATE SET
                card = excluded.card,
                special_title = excluded.special_title,
                role = excluded.role,
                joined_at = excluded.joined_at,
                last_sent_at = excluded.last_sent_at,
                mute_until = excluded.mute_until
            "#,
        )
        .bind(&member.group_id)
        .bind(&member.user_id)
        .bind(&member.card)
        .bind(&member.title)
        .bind(codecs::group_role_to_db(member.role))
        .bind(member.joined_at as i64)
        .bind(member.last_sent_at as i64)
        .bind(member.mute_until.map(|ts| ts as i64))
        .execute(&mut *tx)
        .await?;

        let default_category = format!("{}:group:default", member.user_id);
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO user_groups (
                owner_user_id, group_id, category_id, joined_at
            ) VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(&member.user_id)
        .bind(&member.group_id)
        .bind(&default_category)
        .bind(member.joined_at as i64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_group_members(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupMemberProfile>, sqlx::Error> {
        let rows = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            SELECT group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
            FROM group_members
            WHERE group_id = ?1
            ORDER BY user_id ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_group_member(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            SELECT group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
            FROM group_members
            WHERE group_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn set_group_member_mute(
        &self,
        group_id: &str,
        user_id: &str,
        mute_until: Option<u64>,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            UPDATE group_members
            SET mute_until = ?3
            WHERE group_id = ?1 AND user_id = ?2
            RETURNING group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .bind(mute_until.map(|t| t as i64))
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn remove_group_member(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            DELETE FROM group_members
            WHERE group_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM user_groups
            WHERE owner_user_id = ?1 AND group_id = ?2
            "#,
        )
        .bind(user_id)
        .bind(group_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_group_member_role(
        &self,
        group_id: &str,
        user_id: &str,
        role: GroupRole,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            UPDATE group_members
            SET role = ?3
            WHERE group_id = ?1 AND user_id = ?2
            RETURNING group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .bind(codecs::group_role_to_db(role))
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn update_group_member_title(
        &self,
        group_id: &str,
        user_id: &str,
        title: String,
    ) -> Result<Option<GroupMemberProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupMemberRow>(
            r#"
            UPDATE group_members
            SET special_title = ?3
            WHERE group_id = ?1 AND user_id = ?2
            RETURNING group_id, user_id, card, special_title, role, joined_at, last_sent_at, mute_until
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .bind(title)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn update_group_name(
        &self,
        group_id: &str,
        group_name: String,
    ) -> Result<Option<GroupProfile>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupRow>(
            r#"
            UPDATE chat_groups
            SET group_name = ?2,
                updated_at = unixepoch() * 1000
            WHERE group_id = ?1
            RETURNING
                group_id,
                group_name,
                group_owner_user_id AS owner_user_id,
                member_count,
                max_member_count,
                group_status
            "#,
        )
        .bind(group_id)
        .bind(group_name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }

    pub async fn delete_group(&self, group_id: &str) -> Result<bool, sqlx::Error> {
        let affected = sqlx::query(
            r#"
            UPDATE chat_groups
            SET group_status = 'dissolved',
                dissolved_at = unixepoch() * 1000,
                updated_at = unixepoch() * 1000
            WHERE group_id = ?1
            "#,
        )
        .bind(group_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(affected > 0)
    }

    pub async fn update_group_member_last_sent_at(
        &self,
        group_id: &str,
        user_id: &str,
        last_sent_at: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE group_members
            SET last_sent_at = ?3
            WHERE group_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .bind(last_sent_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_group_whole_mute(
        &self,
        group_id: &str,
        muted: bool,
        mute_until: Option<u64>,
        operator_user_id: &str,
        updated_at: u64,
    ) -> Result<GroupWholeMuteState, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupWholeMuteRow>(
            r#"
            UPDATE chat_groups
            SET is_whole_muted = ?2,
                mute_until = ?3,
                mute_operator_user_id = ?4,
                updated_at = ?5
            WHERE group_id = ?1
            RETURNING group_id, is_whole_muted AS muted, mute_until, mute_operator_user_id AS operator_user_id, updated_at
            "#,
        )
        .bind(group_id)
        .bind(muted)
        .bind(mute_until.map(|t| t as i64))
        .bind(operator_user_id)
        .bind(updated_at as i64)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    pub async fn get_group_whole_mute(
        &self,
        group_id: &str,
    ) -> Result<Option<GroupWholeMuteState>, sqlx::Error> {
        let row = sqlx::query_as::<_, GroupWholeMuteRow>(
            r#"
            SELECT group_id, is_whole_muted AS muted, mute_until, mute_operator_user_id AS operator_user_id, updated_at
            FROM chat_groups
            WHERE group_id = ?1
            "#,
        )
        .bind(group_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(TryInto::try_into).transpose()
    }
}
