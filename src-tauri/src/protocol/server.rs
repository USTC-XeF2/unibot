use std::sync::Arc;

use crate::error::AppResult;
use crate::protocol::backend::ProtocolBackend;
use crate::protocol::types::{BotRuntimeContext, MilkyAdapter};

/// Spawn a protocol HTTP server on the given listener.
///
/// Returns a oneshot sender for graceful shutdown and a join handle for the server task.
/// Full implementation will be provided in Task 13.
pub async fn spawn_server(
    listener: tokio::net::TcpListener,
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<MilkyAdapter>,
) -> AppResult<(
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
)> {
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();

    // Placeholder: immediately resolve the join handle.
    // Task 13 will replace this with the actual axum server.
    let handle = tokio::spawn(async move {
        let _ = listener;
        let _ = context;
        let _ = backend;
        let _ = adapter;
    });

    Ok((shutdown_tx, handle))
}
