use std::convert::Infallible;
use std::sync::Arc;

use axum::response::sse::Event;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Sse},
    routing::{get, post},
};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::AppResult;
use crate::models::InternalEvent;
use crate::protocol::PacketRecorder;
use crate::protocol::backend::ProtocolBackend;
use crate::protocol::types::{ApiRequest, ApiResponse, BotRuntimeContext, ProtocolAdapter};

/// Server state shared across all request handlers.
#[derive(Clone)]
struct ServerState {
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<dyn ProtocolAdapter>,
    recorder: Option<Arc<PacketRecorder>>,
    session_id: String,
}

/// Query parameters used for auth token extraction.
#[derive(serde::Deserialize)]
struct AuthQuery {
    access_token: Option<String>,
}

/// Extract Bearer token from Authorization header or query param.
fn extract_token(headers: &axum::http::HeaderMap, query: &AuthQuery) -> Option<String> {
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    query.access_token.clone()
}

/// SSE event stream handler (GET /event).
///
/// Authenticates, subscribes to backend events, converts each `InternalEvent`
/// to a `ProtocolEvent` via the adapter, and streams as SSE.
/// Echo messages (where `origin_bot_id == current_bot_id`) are filtered out.
async fn event_handler(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<AuthQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let token = extract_token(&headers, &query).ok_or(StatusCode::UNAUTHORIZED)?;
    if token != state.context.access_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let rx = state
        .backend
        .subscribe_events(&state.context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let adapter = state.adapter.clone();
    let bot_id = state.context.bot_id.clone();
    let recorder = state.recorder.clone();
    let session_id = state.session_id.clone();
    let context = state.context.clone();

    let stream = BroadcastStream::new(rx)
        .then(move |result| {
            let adapter = adapter.clone();
            let bot_id = bot_id.clone();
            let recorder = recorder.clone();
            let session_id = session_id.clone();
            let context = context.clone();
            async move {
                match result {
                    Ok(event) => {
                        // Filter out echo messages where origin_bot_id == current_bot_id
                        if let InternalEvent::Message { origin_bot_id, .. } = &event {
                            if origin_bot_id.as_ref() == Some(&bot_id) {
                                return None;
                            }
                        }

                        // Record event via PacketRecorder if present
                        if let Some(_rec) = recorder.as_ref() {
                            // Placeholder: recording will be implemented in the features plan
                            let _ = &session_id;
                        }

                        let protocol_event = adapter.adapt_event(&event, &context)?;
                        let json = serde_json::to_string(&protocol_event).ok()?;
                        Some(Ok::<_, Infallible>(
                            Event::default().event("milky_event").data(json),
                        ))
                    }
                    Err(_) => None,
                }
            }
        })
        .filter_map(|opt| opt);

    Ok(Sse::new(stream))
}

/// API call handler (POST /api/:api).
///
/// Authenticates, parses the JSON body as API params, dispatches to the
/// backend, and returns an `ApiResponse` (HTTP 200 even on errors).
async fn api_handler(
    State(state): State<Arc<ServerState>>,
    Path(api): Path<String>,
    Query(query): Query<AuthQuery>,
    headers: axum::http::HeaderMap,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let token = match extract_token(&headers, &query) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(ApiResponse::failed(-401, "unauthorized")),
            );
        }
    };
    if token != state.context.access_token {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::failed(-401, "unauthorized")),
        );
    }

    // Record request via PacketRecorder if present
    if let Some(_rec) = state.recorder.as_ref() {
        let _ = &state.session_id;
    }

    let request = ApiRequest {
        api_name: api,
        params: body,
    };

    match state.backend.call_api(&state.context, request).await {
        Ok(data) => {
            // Record response via PacketRecorder if present
            if let Some(_rec) = state.recorder.as_ref() {
                let _ = &state.session_id;
            }
            (StatusCode::OK, axum::Json(ApiResponse::ok(data)))
        }
        Err(err) => {
            let (retcode, message) = state.adapter.adapt_error(&err);
            // Record error response via PacketRecorder if present
            if let Some(_rec) = state.recorder.as_ref() {
                let _ = &state.session_id;
            }
            (
                StatusCode::OK,
                axum::Json(ApiResponse::failed(retcode, message)),
            )
        }
    }
}

/// Spawn an axum protocol server on the given TCP listener.
///
/// Returns a oneshot sender for graceful shutdown and a join handle for the
/// server task. The server handles:
/// - `GET /event`  – authenticated SSE event stream
/// - `POST /api/:api` – authenticated API dispatch
pub async fn spawn_server(
    listener: tokio::net::TcpListener,
    context: BotRuntimeContext,
    backend: Arc<dyn ProtocolBackend>,
    adapter: Arc<dyn ProtocolAdapter>,
    recorder: Option<Arc<PacketRecorder>>,
    session_id: String,
) -> AppResult<(
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
)> {
    let state = Arc::new(ServerState {
        context,
        backend,
        adapter,
        recorder,
        session_id,
    });

    let app = Router::new()
        .route("/event", get(event_handler))
        .route("/api/:api", post(api_handler))
        .with_state(state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let graceful = server.with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = graceful.await {
            eprintln!("protocol server error: {e}");
        }
    });

    Ok((shutdown_tx, handle))
}
