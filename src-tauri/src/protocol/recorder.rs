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

/// Borrowed fields for a recorded protocol event. Groups the varying inputs to
/// [`PacketRecorder::record_event`] so the public method stays within a
/// readable argument count; the fixed protocol/direction/error fields are
/// filled in by `record_event` itself.
#[derive(Clone, Copy)]
pub struct EventRecord<'a> {
    pub bot_id: &'a str,
    pub profile_id: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub event_type: &'a str,
    pub related_object_type: Option<&'a str>,
    pub related_object_id: Option<&'a str>,
}

/// Borrowed identity and classification fields shared by every packet write.
///
/// Threading these as one struct (instead of 9 positional arguments) keeps the
/// record/insert call sites readable and removes the same-typed-argument
/// ordering hazard between the several `Option<&str>` fields.
#[derive(Clone, Copy)]
struct PacketMeta<'a> {
    bot_id: &'a str,
    profile_id: Option<&'a str>,
    session_id: Option<&'a str>,
    protocol_type: &'a str,
    direction: &'a str,
    action_name: &'a str,
    related_object_type: Option<&'a str>,
    related_object_id: Option<&'a str>,
    is_error: bool,
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
            PacketMeta {
                bot_id,
                profile_id,
                session_id,
                protocol_type: "milky",
                direction: "receive",
                action_name,
                related_object_type: None,
                related_object_id: None,
                is_error: false,
            },
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
            PacketMeta {
                bot_id,
                profile_id,
                session_id,
                protocol_type: "milky",
                direction: "send",
                action_name,
                related_object_type: None,
                related_object_id: None,
                is_error,
            },
            data,
        )
        .await
    }

    /// Record a protocol event.
    pub async fn record_event(
        &self,
        event: EventRecord<'_>,
        data: &serde_json::Value,
    ) -> AppResult<String> {
        self.record(
            PacketMeta {
                bot_id: event.bot_id,
                profile_id: event.profile_id,
                session_id: event.session_id,
                protocol_type: "milky",
                direction: "receive",
                action_name: event.event_type,
                related_object_type: event.related_object_type,
                related_object_id: event.related_object_id,
                is_error: false,
            },
            data,
        )
        .await
    }

    /// Core recording logic: atomic file write + tiered database indexing.
    async fn record(&self, meta: PacketMeta<'_>, data: &serde_json::Value) -> AppResult<String> {
        let packet_id = utils::new_db_id();
        let tier = classify_tier(meta.action_name, meta.is_error);

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
                self.insert_db(&packet_id, meta, &relative_path).await?;
            }
            Tier::Normal => {
                let item = BatchItem {
                    packet_id: packet_id.clone(),
                    bot_id: meta.bot_id.to_string(),
                    profile_id: meta.profile_id.map(|s| s.to_string()),
                    protocol_type: meta.protocol_type.to_string(),
                    direction: meta.direction.to_string(),
                    action_name: meta.action_name.to_string(),
                    file_path: relative_path,
                    related_object_type: meta.related_object_type.map(|s| s.to_string()),
                    related_object_id: meta.related_object_id.map(|s| s.to_string()),
                    is_error: if meta.is_error { 1 } else { 0 },
                    session_id: meta.session_id.map(|s| s.to_string()),
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

    async fn insert_db(
        &self,
        packet_id: &str,
        meta: PacketMeta<'_>,
        file_path: &str,
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
        .bind(meta.bot_id)
        .bind(meta.profile_id)
        .bind(meta.protocol_type)
        .bind(meta.direction)
        .bind(meta.action_name)
        .bind(file_path)
        .bind(meta.related_object_type)
        .bind(meta.related_object_id)
        .bind(if meta.is_error { 1 } else { 0 })
        .bind(meta.session_id)
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
