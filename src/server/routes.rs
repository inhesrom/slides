use std::path::Path;

use axum::{
    Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
};
use tokio::sync::broadcast;

use super::SharedDeck;

#[derive(Clone)]
pub struct AppState {
    pub deck: SharedDeck,
    pub tx: broadcast::Sender<String>,
}

/// Build the Axum router with index, websocket, and static file routes.
pub fn create_router(
    deck: SharedDeck,
    tx: broadcast::Sender<String>,
    file: &Path,
) -> Router {
    let state = AppState { deck, tx };

    // Serve assets from the markdown file's parent directory
    let assets_dir = file
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let serve_dir = tower_http::services::ServeDir::new(assets_dir);

    Router::new()
        .route("/", get(index))
        .route("/ws", get(ws_handler))
        .fallback_service(serve_dir)
        .with_state(state)
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let deck = state.deck.read().await;
    Html(deck.html.clone())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg.into())).await.is_err() {
            break;
        }
    }
}
