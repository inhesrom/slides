use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use tokio::sync::{RwLock, broadcast};

use crate::parser;
use crate::render::{self, RenderedDeck};

/// Reload the deck from disk, parse, and render it.
fn reload_deck(file: &PathBuf) -> Result<RenderedDeck> {
    let input = std::fs::read_to_string(file)?;
    let deck = parser::parse(&input)?;
    render::render_deck(&deck)
}

/// Watch a markdown file for changes and broadcast reload events.
pub fn watch(
    file: PathBuf,
    shared: Arc<RwLock<RenderedDeck>>,
    tx: broadcast::Sender<String>,
) -> Result<()> {
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), notify_tx)?;

    debouncer
        .watcher()
        .watch(&file, RecursiveMode::NonRecursive)?;

    tracing::info!("Watching {} for changes", file.display());

    for result in notify_rx {
        match result {
            Ok(_events) => {
                tracing::info!("File changed, reloading...");
                match reload_deck(&file) {
                    Ok(rendered) => {
                        let rt = tokio::runtime::Handle::current();
                        rt.block_on(async {
                            *shared.write().await = rendered;
                        });
                        let _ = tx.send(r#"{"type":"reload"}"#.to_string());
                    }
                    Err(e) => tracing::error!("Reload error: {}", e),
                }
            }
            Err(e) => tracing::error!("Watch error: {:?}", e),
        }
    }

    Ok(())
}
