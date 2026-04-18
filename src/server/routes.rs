use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use axum::{
    Router,
    extract::{Query, State},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::header,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
};
use serde::Deserialize;
use tokio::sync::{Mutex, broadcast};

use super::SharedDeck;
use crate::editor;
use crate::help;
use crate::parser;
use crate::presenter;
use crate::render;

/// Duration after an editor write during which reloads are treated as self-triggered.
const WRITE_SUPPRESS_WINDOW: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone)]
pub struct AppState {
    pub deck: SharedDeck,
    pub tx: broadcast::Sender<String>,
    pub deck_title: String,
    pub file_path: PathBuf,
    /// Timestamp of the last editor-initiated write.
    /// Any reload within WRITE_SUPPRESS_WINDOW is treated as self-triggered.
    pub last_write_time: Arc<Mutex<Option<Instant>>>,
}

/// Build the Axum router with index, websocket, presenter, and static file routes.
pub fn create_router(
    deck: SharedDeck,
    tx: broadcast::Sender<String>,
    file: &Path,
    deck_title: String,
    editor_mode: bool,
) -> Router {
    let state = AppState {
        deck,
        tx,
        deck_title,
        file_path: file.to_path_buf(),
        last_write_time: Arc::new(Mutex::new(None)),
    };

    let assets_dir = file.parent().unwrap_or(Path::new(".")).to_path_buf();

    let serve_dir = tower_http::services::ServeDir::new(assets_dir);

    let mut router = Router::new()
        .route("/", get(index))
        .route("/help", get(help_view))
        .route("/syntax.md", get(syntax_md_view))
        .route("/presenter", get(presenter_view))
        .route("/ws", get(ws_handler));

    if editor_mode {
        router = router
            .route("/edit", get(editor_view))
            .route("/ws/edit", get(edit_ws_handler))
            .route("/api/upload", post(upload_handler));
    }

    router.fallback_service(serve_dir).with_state(state)
}

#[derive(Deserialize, Default)]
struct IndexQuery {
    /// When present (`?editor=1`), serve the editor-preview variant that
    /// includes hidden slides marked with `data-hidden="true"`.
    editor: Option<String>,
}

async fn index(
    State(state): State<AppState>,
    Query(q): Query<IndexQuery>,
) -> impl IntoResponse {
    let deck = state.deck.read().await;
    if q.editor.is_some() {
        Html(deck.editor_html.clone())
    } else {
        Html(deck.html.clone())
    }
}

async fn help_view() -> impl IntoResponse {
    Html(help::help_html())
}

async fn syntax_md_view() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        help::SYNTAX_MD,
    )
}

async fn presenter_view(State(state): State<AppState>) -> impl IntoResponse {
    Html(presenter::presenter_html(&state.deck_title))
}

async fn editor_view() -> impl IntoResponse {
    Html(editor::page::editor_html())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();

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
                        let _ = state.tx.send(text.to_string());
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

// --- Editor WebSocket ---

async fn edit_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_edit_ws(socket, state))
}

async fn handle_edit_ws(mut socket: WebSocket, state: AppState) {
    // Send initial editor state
    match load_editor_state(&state.file_path) {
        Ok(deck) => {
            let init_msg = serde_json::json!({
                "type": "init",
                "deck": deck,
            });
            if socket
                .send(Message::Text(init_msg.to_string().into()))
                .await
                .is_err()
            {
                return;
            }
        }
        Err(e) => {
            let err_msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to load file: {}", e),
            });
            let _ = socket
                .send(Message::Text(err_msg.to_string().into()))
                .await;
            return;
        }
    }

    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            // Forward reload broadcasts to editor (for preview refresh)
            msg = rx.recv() => {
                match msg {
                    Ok(text) => {
                        // Check if this reload was self-triggered (within suppression window)
                        let is_self_triggered = {
                            let write_time = state.last_write_time.lock().await;
                            write_time.is_some_and(|t| t.elapsed() < WRITE_SUPPRESS_WINDOW)
                        };

                        if is_self_triggered {
                            // Self-triggered: just tell the editor to refresh preview
                            let saved_msg = serde_json::json!({ "type": "saved" });
                            if socket.send(Message::Text(saved_msg.to_string().into())).await.is_err() {
                                break;
                            }
                        } else {
                            // External change: forward the reload
                            if socket.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            // Handle messages from editor client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match handle_editor_message(&text, &state).await {
                            Ok(Some(reply)) => {
                                if socket.send(Message::Text(reply.to_string().into())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(None) => {
                                // Watcher will fire and deliver the ack via broadcast.
                            }
                            Err(e) => {
                                let err_msg = serde_json::json!({
                                    "type": "error",
                                    "message": format!("{}", e),
                                });
                                if socket.send(Message::Text(err_msg.to_string().into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

fn load_editor_state(file_path: &Path) -> anyhow::Result<editor::types::EditorDeck> {
    let input = std::fs::read_to_string(file_path)?;
    editor::deck_to_editor(&input)
}

/// Handle an editor WebSocket message.
///
/// Returns `Ok(Some(reply))` when the caller should send `reply` directly to
/// the client (e.g. a `"saved"` ack when no file was written, or rendered
/// `"preview-html"` for a live-preview request). Returns `Ok(None)` when the
/// file watcher will deliver the acknowledgement via broadcast instead.
async fn handle_editor_message(
    text: &str,
    state: &AppState,
) -> anyhow::Result<Option<serde_json::Value>> {
    let msg: serde_json::Value = serde_json::from_str(text)?;

    match msg.get("type").and_then(|t| t.as_str()) {
        Some("save") => {
            let deck: editor::types::EditorDeck = serde_json::from_value(
                msg.get("deck")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing deck field"))?,
            )?;

            let markdown = editor::serialize::serialize_deck(&deck);

            // Only write if content changed (prevents watcher loops)
            let current = std::fs::read_to_string(&state.file_path).unwrap_or_default();
            if markdown != current {
                // Mark the write timestamp so watcher reloads are suppressed
                {
                    let mut write_time = state.last_write_time.lock().await;
                    *write_time = Some(Instant::now());
                }
                std::fs::write(&state.file_path, &markdown)?;
                tracing::debug!("Editor saved {}", state.file_path.display());
                // Watcher will fire and deliver the ack via broadcast.
                Ok(None)
            } else {
                // No file change — ack immediately.
                Ok(Some(serde_json::json!({ "type": "saved" })))
            }
        }
        Some("preview") => {
            let deck: editor::types::EditorDeck = serde_json::from_value(
                msg.get("deck")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing deck field"))?,
            )?;
            let slide_index = msg
                .get("slide")
                .and_then(|s| s.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Missing slide field"))?
                as usize;

            let markdown = editor::serialize::serialize_deck(&deck);
            let parsed = parser::parse(&markdown)?;
            let html = render::render_slide_html(&parsed, slide_index)?;

            Ok(Some(serde_json::json!({
                "type": "preview-html",
                "slide": slide_index,
                "html": html,
            })))
        }
        _ => Ok(None),
    }
}

// --- File Upload ---

async fn upload_handler(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> impl IntoResponse {
    let assets_dir = state
        .file_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("assets");

    // Create assets directory if needed
    if let Err(e) = std::fs::create_dir_all(&assets_dir) {
        return Json(serde_json::json!({
            "error": format!("Failed to create assets directory: {}", e),
        }));
    }

    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field
            .file_name()
            .map(|f| sanitize_filename(f))
            .unwrap_or_else(|| "upload".to_string());

        if let Ok(data) = field.bytes().await {
            let dest = assets_dir.join(&filename);
            if let Err(e) = std::fs::write(&dest, &data) {
                return Json(serde_json::json!({
                    "error": format!("Failed to write file: {}", e),
                }));
            }

            let relative_path = format!("assets/{}", filename);
            tracing::info!("Uploaded {} ({} bytes)", relative_path, data.len());

            return Json(serde_json::json!({
                "path": relative_path,
            }));
        }
    }

    Json(serde_json::json!({
        "error": "No file uploaded",
    }))
}

/// Sanitize a filename: keep only safe characters.
fn sanitize_filename(name: &str) -> String {
    let name = name
        .replace(['/', '\\'], "")
        .replace("..", "")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>();

    if name.is_empty() {
        "upload".to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor;

    // --- Save handler tests ---

    #[tokio::test]
    async fn test_save_writes_file_when_content_changes() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(&file_path, "# Old content\n").unwrap();

        let state = AppState {
            deck: Arc::new(tokio::sync::RwLock::new(
                crate::render::render_deck(&crate::parser::parse("# Old\n").unwrap()).unwrap(),
            )),
            tx: broadcast::channel(16).0,
            deck_title: "Test".to_string(),
            file_path: file_path.clone(),
            last_write_time: Arc::new(Mutex::new(None)),
        };

        let deck = editor::types::EditorDeck {
            config: editor::types::EditorConfig::from(&crate::parser::frontmatter::DeckConfig::default()),
            slides: vec![editor::types::EditorSlide {
                content: "# New content".to_string(),
                ..editor::types::EditorSlide::default()
            }],
        };

        let msg = serde_json::json!({ "type": "save", "deck": deck });
        let result = handle_editor_message(&msg.to_string(), &state).await.unwrap();

        // Should return None (watcher will handle ack)
        assert!(result.is_none(), "Should return None when file was written");
        // File should be updated
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# New content"));
        // Write time should be set
        assert!(state.last_write_time.lock().await.is_some());
    }

    #[tokio::test]
    async fn test_save_returns_immediate_ack_when_no_change() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");

        // Write the exact content that the serializer would produce
        let deck = editor::types::EditorDeck {
            config: editor::types::EditorConfig::from(&crate::parser::frontmatter::DeckConfig::default()),
            slides: vec![editor::types::EditorSlide {
                content: "# Same content".to_string(),
                ..editor::types::EditorSlide::default()
            }],
        };
        let markdown = editor::serialize::serialize_deck(&deck);
        std::fs::write(&file_path, &markdown).unwrap();

        let state = AppState {
            deck: Arc::new(tokio::sync::RwLock::new(
                crate::render::render_deck(&crate::parser::parse("# Old\n").unwrap()).unwrap(),
            )),
            tx: broadcast::channel(16).0,
            deck_title: "Test".to_string(),
            file_path: file_path.clone(),
            last_write_time: Arc::new(Mutex::new(None)),
        };

        let msg = serde_json::json!({ "type": "save", "deck": deck });
        let result = handle_editor_message(&msg.to_string(), &state).await.unwrap();

        // Should return Some({"type":"saved"}) (immediate ack, no file write)
        let reply = result.expect("expected immediate ack when content unchanged");
        assert_eq!(reply["type"], "saved");
        // Write time should NOT be set
        assert!(state.last_write_time.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_preview_returns_rendered_slide_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        // Seed with something so the preview path doesn't race a missing file.
        std::fs::write(&file_path, "# Existing\n").unwrap();
        let before = std::fs::read_to_string(&file_path).unwrap();

        let state = AppState {
            deck: Arc::new(tokio::sync::RwLock::new(
                crate::render::render_deck(&crate::parser::parse("# Existing\n").unwrap()).unwrap(),
            )),
            tx: broadcast::channel(16).0,
            deck_title: "Test".to_string(),
            file_path: file_path.clone(),
            last_write_time: Arc::new(Mutex::new(None)),
        };

        let deck = editor::types::EditorDeck {
            config: editor::types::EditorConfig::from(
                &crate::parser::frontmatter::DeckConfig::default(),
            ),
            slides: vec![
                editor::types::EditorSlide {
                    content: "# First".to_string(),
                    ..editor::types::EditorSlide::default()
                },
                editor::types::EditorSlide {
                    content: "# Second".to_string(),
                    ..editor::types::EditorSlide::default()
                },
            ],
        };

        let msg = serde_json::json!({ "type": "preview", "slide": 1, "deck": deck });
        let reply = handle_editor_message(&msg.to_string(), &state)
            .await
            .unwrap()
            .expect("preview should return a reply");

        assert_eq!(reply["type"], "preview-html");
        assert_eq!(reply["slide"], 1);
        let html = reply["html"].as_str().unwrap();
        assert!(html.contains("<h1>Second</h1>"));
        assert!(!html.contains("First"));

        // Preview must never touch the file or the suppression window.
        let after = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(before, after, "preview must not rewrite the file");
        assert!(state.last_write_time.lock().await.is_none());
    }

    // --- Sanitize filename tests ---

    #[test]
    fn test_sanitize_filename_basic() {
        assert_eq!(sanitize_filename("photo.png"), "photo.png");
    }

    #[test]
    fn test_sanitize_filename_path_traversal() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "etcpasswd");
    }

    #[test]
    fn test_sanitize_filename_spaces_special() {
        assert_eq!(sanitize_filename("my file (1).png"), "myfile1.png");
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert_eq!(sanitize_filename(""), "upload");
    }

    #[test]
    fn test_sanitize_filename_unicode() {
        // Non-ASCII alphanumeric chars should pass through
        let result = sanitize_filename("café.png");
        assert!(result.contains("caf"));
        assert!(result.ends_with(".png"));
    }
}
