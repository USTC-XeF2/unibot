use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::utils;

/// Packet recorder with atomic file write and database indexing.
///
/// Each packet is written as a JSON file to the filesystem and indexed
/// in the `protocol_packets` table for efficient querying.
#[derive(Clone)]
pub struct PacketRecorder {
    app_data_dir: PathBuf,
    pool: SqlitePool,
}

impl PacketRecorder {
    pub fn new(app_data_dir: PathBuf, pool: SqlitePool) -> Self {
        Self { app_data_dir, pool }
    }

    /// Record an incoming request (framework -> UniBot).
    pub async fn record_request(
        &self,
        bot_id: &str,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        action_name: &str,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            bot_id,
            profile_id,
            session_id,
            "milky",
            "receive",
            action_name,
            None,
            None,
            false,
            data,
        )
        .await
    }

    /// Record an outgoing response (UniBot -> framework).
    pub async fn record_response(
        &self,
        bot_id: &str,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        action_name: &str,
        is_error: bool,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            bot_id,
            profile_id,
            session_id,
            "milky",
            "send",
            action_name,
            None,
            None,
            is_error,
            data,
        )
        .await
    }

    /// Record a protocol event.
    pub async fn record_event(
        &self,
        bot_id: &str,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        event_type: &str,
        related_object_type: Option<&str>,
        related_object_id: Option<&str>,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            bot_id,
            profile_id,
            session_id,
            "milky",
            "receive",
            event_type,
            related_object_type,
            related_object_id,
            false,
            data,
        )
        .await
    }

    /// Core recording logic: atomic file write + database index.
    async fn record(
        &self,
        bot_id: &str,
        profile_id: Option<&str>,
        session_id: Option<&str>,
        protocol_type: &str,
        direction: &str,
        action_name: &str,
        related_object_type: Option<&str>,
        related_object_id: Option<&str>,
        is_error: bool,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        let packet_id = utils::new_db_id();

        // Build date-based directory: {app_data_dir}/packets/YYYY-MM-DD/
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let packets_dir = self.app_data_dir.join("packets").join(&today);
        tokio::fs::create_dir_all(&packets_dir)
            .await
            .map_err(|e| AppError::Storage(format!("failed to create packets dir: {e}")))?;

        let file_path = packets_dir.join(format!("{packet_id}.json"));
        let temp_path = packets_dir.join(format!(".{packet_id}.tmp"));

        // Serialize JSON
        let json_bytes = serde_json::to_vec_pretty(data)
            .map_err(|e| AppError::Internal(format!("failed to serialize packet data: {e}")))?;

        // Atomic write: write to temp file, then rename
        tokio::fs::write(&temp_path, &json_bytes)
            .await
            .map_err(|e| AppError::Storage(format!("failed to write packet temp file: {e}")))?;

        tokio::fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| {
                let _ = std::fs::remove_file(&temp_path);
                AppError::Storage(format!("failed to rename packet file: {e}"))
            })?;

        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| AppError::Internal("packet file path is not valid UTF-8".to_string()))?;

        // Insert index record into database
        let result = sqlx::query(
            r#"
            INSERT INTO protocol_packets (
                packet_id, bot_id, profile_id, protocol_type, direction,
                action_name, file_path, related_object_type, related_object_id,
                is_error, session_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&packet_id)
        .bind(bot_id)
        .bind(profile_id)
        .bind(protocol_type)
        .bind(direction)
        .bind(action_name)
        .bind(file_path_str)
        .bind(related_object_type)
        .bind(related_object_id)
        .bind(if is_error { 1 } else { 0 })
        .bind(session_id)
        .bind(utils::now_ts() as i64)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            // Best-effort cleanup of the written file on DB failure
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(AppError::Storage(format!(
                "failed to index packet in database: {e}"
            )));
        }

        Ok(packet_id)
    }
}
