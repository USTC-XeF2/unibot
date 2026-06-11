use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::persistence::{PacketRepo, ProtocolPacketRecord};

#[derive(Clone)]
pub struct PacketService {
    repo: PacketRepo,
}

impl PacketService {
    pub fn new(repo: PacketRepo) -> Self {
        Self { repo }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_packets(
        &self,
        bot_id: Option<String>,
        direction: Option<String>,
        action_name: Option<String>,
        since: Option<u64>,
        until: Option<u64>,
        is_error: Option<bool>,
        before: Option<u64>,
        limit: i64,
    ) -> AppResult<Vec<ProtocolPacketRecord>> {
        let limit = limit.max(1).min(1000);
        self.repo
            .list_packets(
                bot_id.as_deref(),
                direction.as_deref(),
                action_name.as_deref(),
                since,
                until,
                is_error,
                before,
                limit,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn read_packet(&self, packet_id: String, app_data_dir: PathBuf) -> AppResult<String> {
        let packet = self
            .repo
            .get_packet_by_id(&packet_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("packet {} not found", packet_id)))?;

        let file_path = app_data_dir.join(&packet.file_path);
        let canonical = tokio::fs::canonicalize(&file_path)
            .await
            .map_err(|e| AppError::storage(format!("packet file not found: {e}")))?;
        let allowed_prefix = tokio::fs::canonicalize(&app_data_dir)
            .await
            .map_err(|e| AppError::storage(format!("app data dir not accessible: {e}")))?;

        if !canonical.starts_with(&allowed_prefix) {
            return Err(AppError::validation(
                "packet file path escapes app data directory",
            ));
        }

        tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| AppError::storage(format!("failed to read packet file: {e}")))
    }
}
