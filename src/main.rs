mod editor;
mod help;
mod layout;
mod parser;
mod presenter;
mod render;
mod server;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "slides", about = "Markdown presentations, done right")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Live preview with hot reload
    Serve {
        /// Path to the markdown file
        file: PathBuf,
        /// Port to serve on
        #[arg(short, long, default_value = "3030")]
        port: u16,
        /// Open browser automatically
        #[arg(long, default_value = "true")]
        open: bool,
    },
    /// Export to static HTML or PDF
    Export {
        /// Path to the markdown file
        file: PathBuf,
        /// Output format
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Fullscreen presenter mode with notes
    Present {
        /// Path to the markdown file
        file: PathBuf,
        /// Port to serve on
        #[arg(short, long, default_value = "3030")]
        port: u16,
    },
    /// Create a new presentation from a starter template
    Init {
        /// Output file path
        #[arg(default_value = "presentation.md")]
        file: PathBuf,
    },
    /// Open the visual editor in the browser
    Edit {
        /// Path to the markdown file
        file: PathBuf,
        /// Port to serve on
        #[arg(short, long, default_value = "3030")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve { file, port, open } => {
            server::serve(file, port, open, false).await?;
        }
        Command::Export {
            file,
            format,
            output,
        } => {
            render::export::export(&file, &format, output.as_deref())?;
        }
        Command::Present { file, port } => {
            // Open presenter view, audience gets the root URL
            let url = format!("http://localhost:{}/presenter", port);
            let _ = open::that(&url);
            server::serve(file, port, false, false).await?;
        }
        Command::Edit { file, port } => {
            if !file.exists() {
                // Create a minimal starter file
                std::fs::write(
                    &file,
                    "---\ntitle: My Presentation\ntheme: minimal\naspect: \"16:9\"\ntransition: slide\n---\n\n# My Presentation\n",
                )?;
                tracing::info!("Created {}", file.display());
            }
            let url = format!("http://localhost:{}/edit", port);
            let _ = open::that(&url);
            server::serve(file, port, false, true).await?;
        }
        Command::Init { file } => {
            if file.exists() {
                anyhow::bail!("{} already exists — refusing to overwrite", file.display());
            }
            std::fs::write(&file, help::INIT_TEMPLATE)?;
            tracing::info!("Created {}", file.display());
            println!("Created {}", file.display());
            println!("Run `slides serve {}` to preview.", file.display());
        }
    }

    Ok(())
}
