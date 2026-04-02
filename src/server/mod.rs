pub mod routes;
pub mod watcher;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::parser;
use crate::render::{self, RenderedDeck};

pub type SharedDeck = Arc<RwLock<RenderedDeck>>;

/// Start the presentation server with live reload.
pub async fn serve(file: PathBuf, port: u16, open: bool, editor_mode: bool) -> Result<()> {
    let input = std::fs::read_to_string(&file)?;
    let deck = parser::parse(&input)?;
    let rendered = render::render_deck(&deck)?;

    let shared = Arc::new(RwLock::new(rendered));
    let (tx, _rx) = tokio::sync::broadcast::channel::<String>(16);

    // Capture the Tokio runtime handle before spawning the watcher thread,
    // since std::thread::spawn doesn't have access to the Tokio runtime context.
    let rt_handle = tokio::runtime::Handle::current();
    let watcher_file = file.clone();
    let watcher_shared = shared.clone();
    let watcher_tx = tx.clone();
    std::thread::spawn(move || {
        if let Err(e) = watcher::watch(watcher_file, watcher_shared, watcher_tx, rt_handle) {
            tracing::error!("File watcher error: {}", e);
        }
    });

    let deck_title = deck
        .config
        .title
        .clone()
        .unwrap_or_else(|| "Slides".to_string());
    let app = routes::create_router(shared, tx, &file, deck_title, editor_mode);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Serving on http://localhost:{}", port);
    if editor_mode {
        tracing::info!("Editor at http://localhost:{}/edit", port);
    }

    if open {
        let url = format!("http://localhost:{}", port);
        let _ = open::that(&url);
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
