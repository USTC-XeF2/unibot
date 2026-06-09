use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::core::CoreContainer;
use crate::error::{AppError, AppResult};
use crate::persistence::BotRepo;
use crate::protocol::backend::VirtualBackend;
use crate::protocol::server::spawn_server;
use crate::protocol::types::MilkyAdapter;
use crate::protocol::types::{BotConfig, BotRuntimeContext};
use crate::services::ServiceHub;
use crate::utils::now_ts;

/// Manages the lifecycle of running protocol servers — one per bot.
pub struct ProtocolRuntimeManager {
    servers: Mutex<HashMap<String, RunningProtocolServer>>,
    bot_repo: BotRepo,
    service_hub: ServiceHub,
    core: Arc<CoreContainer>,
    app_data_dir: PathBuf,
}

/// A running protocol server entry tracked by the runtime manager.
pub struct RunningProtocolServer {
    pub bot_id: String,
    pub session_id: String,
    pub bound_addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl ProtocolRuntimeManager {
    pub fn new(
        bot_repo: BotRepo,
        service_hub: ServiceHub,
        core: CoreContainer,
        app_data_dir: PathBuf,
    ) -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
            bot_repo,
            service_hub,
            core: Arc::new(core),
            app_data_dir,
        }
    }

    /// Start a protocol server for the given bot.
    ///
    /// 1. Checks if the bot is already running.
    /// 2. Reads the bot's config file.
    /// 3. Binds a TCP listener to the configured port.
    /// 4. Starts a debug session via `BotRepo::start_session`.
    /// 5. Creates a `VirtualBackend` and `MilkyAdapter`.
    /// 6. Spawns the protocol server.
    /// 7. Registers the running server.
    ///
    /// On any failure after session creation, the session is stopped.
    pub async fn start_bot(&self, bot_id: &str) -> AppResult<SocketAddr> {
        let mut servers = self.servers.lock().await;
        if servers.contains_key(bot_id) {
            return Err(AppError::conflict("bot is already running"));
        }

        let bot = self
            .bot_repo
            .get_bot_by_id(bot_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("bot {bot_id} not found")))?;

        let config_str = tokio::fs::read_to_string(&bot.config_path)
            .await
            .map_err(|e| AppError::internal(format!("read config: {e}")))?;
        let config: BotConfig = serde_json::from_str(&config_str)
            .map_err(|e| AppError::internal(format!("parse config: {e}")))?;

        let addr = format!("{}:{}", config.http.host, config.http.port)
            .parse::<SocketAddr>()
            .map_err(|e| AppError::internal(format!("invalid bind address: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            AppError::internal(format!("failed to bind to port {}: {e}", config.http.port))
        })?;
        let bound_addr = listener
            .local_addr()
            .map_err(|e| AppError::internal(format!("local_addr: {e}")))?;

        let session_id = crate::utils::new_db_id();
        let session_name = format!("调试会话 {}", now_ts());

        let session = self
            .bot_repo
            .start_session(&session_id, bot_id, &session_name)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    AppError::conflict("bot is already running or does not exist")
                }
                _ => e.into(),
            })?;

        let backend = Arc::new(VirtualBackend::new(
            self.service_hub.clone(),
            self.core.clone(),
        ));
        let adapter = Arc::new(MilkyAdapter::new());

        let context = BotRuntimeContext {
            bot_id: bot_id.to_string(),
            bound_user_id: bot.bound_user_id,
            access_token: config.access_token,
            listen_addr: bound_addr,
        };

        let (shutdown_tx, join_handle) =
            match spawn_server(listener, context, backend, adapter).await {
                Ok((tx, handle)) => (tx, handle),
                Err(e) => {
                    let _ = self.bot_repo.stop_active_sessions(bot_id).await;
                    return Err(e);
                }
            };

        servers.insert(
            bot_id.to_string(),
            RunningProtocolServer {
                bot_id: bot_id.to_string(),
                session_id: session.session_id,
                bound_addr,
                shutdown_tx: Some(shutdown_tx),
                join_handle,
            },
        );

        Ok(bound_addr)
    }

    /// Stop a running bot's protocol server.
    ///
    /// 1. Removes the server from the registry.
    /// 2. Sends a shutdown signal.
    /// 3. Waits for the server task to complete.
    /// 4. Stops active sessions via `BotRepo`.
    pub async fn stop_bot(&self, bot_id: &str) -> AppResult<()> {
        let mut servers = self.servers.lock().await;
        let running = servers
            .remove(bot_id)
            .ok_or_else(|| AppError::validation("bot is not running"))?;

        if let Some(tx) = running.shutdown_tx {
            let _ = tx.send(());
        }

        let _ = running.join_handle.await;
        self.bot_repo.stop_active_sessions(bot_id).await?;
        Ok(())
    }

    /// Gracefully shut down all running protocol servers.
    pub async fn shutdown_all(&self) {
        let mut servers = self.servers.lock().await;
        for (_, mut running) in servers.drain() {
            if let Some(tx) = running.shutdown_tx.take() {
                let _ = tx.send(());
            }
        }
        // Drop lock before awaiting handles
        let handles: Vec<_> = servers
            .drain()
            .map(|(_, running)| running.join_handle)
            .collect();
        drop(servers);
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Check if a bot's protocol server is currently running.
    pub async fn is_running(&self, bot_id: &str) -> bool {
        let servers = self.servers.lock().await;
        servers.contains_key(bot_id)
    }
}
