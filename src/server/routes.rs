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
use crate::presenter;

#[derive(Clone)]
pub struct AppState {
    pub deck: SharedDeck,
    pub tx: broadcast::Sender<String>,
    pub deck_title: String,
}

/// Build the Axum router with index, websocket, presenter, and static file routes.
pub fn create_router(
    deck: SharedDeck,
    tx: broadcast::Sender<String>,
    file: &Path,
    deck_title: String,
) -> Router {
    let state = AppState {
        deck,
        tx,
        deck_title,
    };

    let assets_dir = file
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let serve_dir = tower_http::services::ServeDir::new(assets_dir);

    Router::new()
        .route("/", get(index))
        .route("/presenter", get(presenter_view))
        .route("/ws", get(ws_handler))
        .fallback_service(serve_dir)
        .with_state(state)
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let deck = state.deck.read().await;
    Html(deck.html.clone())
}

async fn presenter_view(State(state): State<AppState>) -> impl IntoResponse {
    Html(presenter::presenter_html(&state.deck_title))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();

    // Handle incoming messages from the client (e.g., navigation from presenter)
    // and broadcast messages from the file watcher
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Broadcast navigation events from presenter to all clients
                        let _ = state.tx.send(text.to_string());
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
