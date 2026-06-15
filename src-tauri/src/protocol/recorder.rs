use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};
use crate::utils;

/// Normal-tier action names that should be indexed in the database
/// (batched for performance).
const NORMAL_TIER_ACTIONS: &[&str] = &[
    "send_message",
    "receive_message",
    "friend_request",
    "group_request",
    "group_event",
    "poke",
    "recall_message",
    "react_to_message",
    "set_group_whole_mute",
    "mute_group_member",
    "kick_group_member",
    "set_group_member_role",
    "rename_group",
];

/// Tier classification for protocol packet writes.
enum Tier {
    /// Critical: is_error=true — immediate DB + file.
    Critical,
    /// Normal: action_name in whitelist — batched DB + immediate file.
    Normal,
    /// Low: everything else — file only, no DB.
    Low,
}

fn classify_tier(action_name: &str, is_error: bool) -> Tier {
    if is_error {
        return Tier::Critical;
    }
    if NORMAL_TIER_ACTIONS.contains(&action_name) {
        return Tier::Normal;
    }
    Tier::Low
}

/// A single item queued for batched database insertion.
struct BatchItem {
    packet_id: String,
    bot_id: String,
    profile_id: Option<String>,
    protocol_type: String,
    direction: String,
    action_name: String,
    file_path: String,
    related_object_type: Option<String>,
    related_object_id: Option<String>,
    is_error: i32,
    session_id: Option<String>,
    created_at: i64,
}

enum BatchOp {
    Item(Box<BatchItem>),
    Flush(tokio::sync::oneshot::Sender<()>),
}

struct PacketRecorderInner {
    app_data_dir: PathBuf,
    pool: SqlitePool,
    /// Lazily initialized sender to the background flush task.
    /// Initialized on first `record()` call when we are guaranteed
    /// to be inside a Tokio runtime.
    batch_tx: OnceLock<mpsc::UnboundedSender<BatchOp>>,
}

/// Packet recorder with tiered writes:
/// - Critical (error): immediate file + immediate DB.
/// - Normal (whitelist actions): immediate file + batched DB (50 items / 100ms).
/// - Low (heartbeats etc.): file only, no DB.
#[derive(Clone)]
pub struct PacketRecorder {
    inner: Arc<PacketRecorderInner>,
}

impl PacketRecorder {
    pub fn new(app_data_dir: PathBuf, pool: SqlitePool) -> Self {
        Self {
            inner: Arc::new(PacketRecorderInner {
                app_data_dir,
                pool,
                batch_tx: OnceLock::new(),
            }),
        }
    }

    /// Ensures the background flush task is running.
    /// Called lazily from `record()` / `flush()` which are async and
    /// therefore guaranteed to be inside a Tokio runtime.
    fn ensure_batch_tx(&self) -> &mpsc::UnboundedSender<BatchOp> {
        self.inner.batch_tx.get_or_init(|| {
            let (batch_tx, mut batch_rx) = mpsc::unbounded_channel::<BatchOp>();
            let pool_clone = self.inner.pool.clone();

            tokio::spawn(async move {
                let mut buffer = Vec::new();
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

                loop {
                    tokio::select! {
                        Some(op) = batch_rx.recv() => {
                            match op {
                                BatchOp::Item(item) => {
                                    buffer.push(*item);
                                    if buffer.len() >= 50 {
                                        Self::flush_buffer(&pool_clone, &mut buffer).await;
                                    }
                                }
                                BatchOp::Flush(reply) => {
                                    if !buffer.is_empty() {
                                        Self::flush_buffer(&pool_clone, &mut buffer).await;
                                    }
                                    let _ = reply.send(());
                                }
                            }
                        }
                        _ = interval.tick() => {
                            if !buffer.is_empty() {
                                Self::flush_buffer(&pool_clone, &mut buffer).await;
                            }
                        }
                        else => break,
                    }
                }

                // Drain remaining items on channel close
                while let Ok(op) = batch_rx.try_recv() {
                    if let BatchOp::Item(item) = op {
                        buffer.push(*item);
                    }
                }
                if !buffer.is_empty() {
                    Self::flush_buffer(&pool_clone, &mut buffer).await;
                }
            });

            batch_tx
        })
    }

    /// Flush any pending batched writes and wait for completion.
    pub async fn flush(&self) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.ensure_batch_tx().send(BatchOp::Flush(tx));
        let _ = rx.await;
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
    #[allow(clippy::too_many_arguments)]
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

    /// Core recording logic: atomic file write + tiered database indexing.
    #[allow(clippy::too_many_arguments)]
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
        let tier = classify_tier(action_name, is_error);

        // Build date-based directory: {app_data_dir}/packets/YYYY-MM-DD/
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let packets_dir = self.inner.app_data_dir.join("packets").join(&today);
        tokio::fs::create_dir_all(&packets_dir)
            .await
            .map_err(|e| AppError::Storage(format!("failed to create packets dir: {e}")))?;

        let relative_path = format!("packets/{today}/{packet_id}.json");
        let file_path = self.inner.app_data_dir.join(&relative_path);
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

        // All tiers persist file. Database indexing depends on tier.
        match tier {
            Tier::Critical => {
                self.insert_db(
                    &packet_id,
                    bot_id,
                    profile_id,
                    protocol_type,
                    direction,
                    action_name,
                    &relative_path,
                    related_object_type,
                    related_object_id,
                    is_error,
                    session_id,
                )
                .await?;
            }
            Tier::Normal => {
                let item = BatchItem {
                    packet_id: packet_id.clone(),
                    bot_id: bot_id.to_string(),
                    profile_id: profile_id.map(|s| s.to_string()),
                    protocol_type: protocol_type.to_string(),
                    direction: direction.to_string(),
                    action_name: action_name.to_string(),
                    file_path: relative_path,
                    related_object_type: related_object_type.map(|s| s.to_string()),
                    related_object_id: related_object_id.map(|s| s.to_string()),
                    is_error: if is_error { 1 } else { 0 },
                    session_id: session_id.map(|s| s.to_string()),
                    created_at: utils::now_ts() as i64,
                };
                let _ = self.ensure_batch_tx().send(BatchOp::Item(Box::new(item)));
            }
            Tier::Low => {
                // File only; no database indexing.
            }
        }

        Ok(packet_id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_db(
        &self,
        packet_id: &str,
        bot_id: &str,
        profile_id: Option<&str>,
        protocol_type: &str,
        direction: &str,
        action_name: &str,
        file_path: &str,
        related_object_type: Option<&str>,
        related_object_id: Option<&str>,
        is_error: bool,
        session_id: Option<&str>,
    ) -> AppResult<()> {
        let result = sqlx::query(
            r#"
            INSERT INTO protocol_packets (
                packet_id, bot_id, profile_id, protocol_type, direction,
                action_name, file_path, related_object_type, related_object_id,
                is_error, session_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(packet_id)
        .bind(bot_id)
        .bind(profile_id)
        .bind(protocol_type)
        .bind(direction)
        .bind(action_name)
        .bind(file_path)
        .bind(related_object_type)
        .bind(related_object_id)
        .bind(if is_error { 1 } else { 0 })
        .bind(session_id)
        .bind(utils::now_ts() as i64)
        .execute(&self.inner.pool)
        .await;

        if let Err(e) = result {
            return Err(AppError::Storage(format!(
                "failed to index packet in database: {e}"
            )));
        }

        Ok(())
    }

    async fn flush_buffer(pool: &SqlitePool, buffer: &mut Vec<BatchItem>) {
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::warn!(
                    target: "packet_recorder",
                    "failed to begin batch transaction: {e}"
                );
                return;
            }
        };

        for item in buffer.drain(..) {
            if let Err(e) = sqlx::query(
                r#"
                INSERT INTO protocol_packets (
                    packet_id, bot_id, profile_id, protocol_type, direction,
                    action_name, file_path, related_object_type, related_object_id,
                    is_error, session_id, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&item.packet_id)
            .bind(&item.bot_id)
            .bind(&item.profile_id)
            .bind(&item.protocol_type)
            .bind(&item.direction)
            .bind(&item.action_name)
            .bind(&item.file_path)
            .bind(&item.related_object_type)
            .bind(&item.related_object_id)
            .bind(item.is_error)
            .bind(&item.session_id)
            .bind(item.created_at)
            .execute(&mut *tx)
            .await
            {
                tracing::warn!(
                    target: "packet_recorder",
                    "failed to batch insert packet {}: {e}",
                    item.packet_id
                );
            }
        }

        if let Err(e) = tx.commit().await {
            tracing::warn!(
                target: "packet_recorder",
                "failed to commit batch transaction: {e}"
            );
        }
    }
}
