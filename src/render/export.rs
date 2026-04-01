use std::path::Path;

use anyhow::{Result, bail};

use crate::parser;
use crate::render;

pub fn export(file: &Path, format: &str, output: Option<&Path>) -> Result<()> {
    let input = std::fs::read_to_string(file)?;
    let deck = parser::parse(&input)?;
    let rendered = render::render_deck(&deck)?;

    match format {
        "html" => {
            let out_path = output
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| file.with_extension("html"));
            std::fs::write(&out_path, &rendered.html)?;
            tracing::info!("Exported to {}", out_path.display());
        }
        "pdf" => {
            bail!("PDF export requires the `pdf` feature flag and Chrome installed");
        }
        _ => bail!("Unknown export format: {}", format),
    }

    Ok(())
}
