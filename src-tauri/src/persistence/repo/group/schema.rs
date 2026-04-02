use sqlx::SqlitePool;

use super::GroupRepo;

impl GroupRepo {
    pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS groups (
                group_id INTEGER PRIMARY KEY,
                group_name TEXT NOT NULL,
                owner_user_id INTEGER NOT NULL,
                FOREIGN KEY (owner_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_members (
                group_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                card TEXT NOT NULL DEFAULT '',
                title TEXT NOT NULL DEFAULT '',
                role INTEGER NOT NULL DEFAULT 2,
                joined_at INTEGER NOT NULL,
                last_sent_at INTEGER NOT NULL DEFAULT 0,
                mute_until INTEGER,
                PRIMARY KEY (group_id, user_id),
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                request_type INTEGER NOT NULL,
                initiator_user_id INTEGER NOT NULL,
                target_user_id INTEGER,
                comment TEXT,
                state INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                handled_at INTEGER,
                operator_user_id INTEGER,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (initiator_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (target_user_id) REFERENCES users(user_id)
                    ON DELETE SET NULL
                    ON UPDATE CASCADE,
                FOREIGN KEY (operator_user_id) REFERENCES users(user_id)
                    ON DELETE SET NULL
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_whole_mute (
                group_id INTEGER PRIMARY KEY,
                muted INTEGER NOT NULL,
                mute_until INTEGER,
                operator_user_id INTEGER,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (operator_user_id) REFERENCES users(user_id)
                    ON DELETE SET NULL
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_announcements (
                announcement_id TEXT PRIMARY KEY,
                group_id INTEGER NOT NULL,
                sender_user_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                image_url TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (sender_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_files (
                file_id TEXT PRIMARY KEY,
                group_id INTEGER NOT NULL,
                parent_folder_id TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                file_hash TEXT,
                uploader_user_id INTEGER NOT NULL,
                uploaded_at INTEGER NOT NULL,
                expire_at INTEGER,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (uploader_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_folders (
                folder_id TEXT PRIMARY KEY,
                group_id INTEGER NOT NULL,
                parent_folder_id TEXT NOT NULL,
                folder_name TEXT NOT NULL,
                creator_user_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (creator_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_essence_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                message_id INTEGER NOT NULL,
                sender_user_id INTEGER NOT NULL,
                operator_user_id INTEGER NOT NULL,
                is_set INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (message_id) REFERENCES messages(id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (sender_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE,
                FOREIGN KEY (operator_user_id) REFERENCES users(user_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS group_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                payload TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(group_id)
                    ON DELETE CASCADE
                    ON UPDATE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_members_user
            ON group_members(user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_members_group_role
            ON group_members(group_id, role)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_requests_initiator
            ON group_requests(initiator_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_requests_target
            ON group_requests(target_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_requests_group_state_created
            ON group_requests(group_id, state, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_announcements_sender
            ON group_announcements(sender_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_requests_pending_key
            ON group_requests(group_id, request_type, initiator_user_id, target_user_id, state)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_files_uploader
            ON group_files(uploader_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_announcements_group_updated
            ON group_announcements(group_id, updated_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_folders_creator
            ON group_folders(creator_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_events_group_time
            ON group_events(group_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_files_group_uploaded
            ON group_files(group_id, uploaded_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_essence_sender
            ON group_essence_messages(sender_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_essence_operator
            ON group_essence_messages(operator_user_id)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_folders_group_updated
            ON group_folders(group_id, updated_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_group_essence_group_created
            ON group_essence_messages(group_id, created_at)
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS trg_groups_cleanup_interactions_after_delete
            AFTER DELETE ON groups
            FOR EACH ROW
            BEGIN
                DELETE FROM message_reactions
                WHERE source_type = 'group' AND source_id = OLD.group_id;

                DELETE FROM pokes
                WHERE source_type = 'group' AND source_id = OLD.group_id;

                DELETE FROM messages
                WHERE source_type = 'group' AND source_id = OLD.group_id;
            END;
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
