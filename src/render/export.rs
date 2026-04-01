use std::path::Path;

use anyhow::{Result, bail};

use crate::parser;
use crate::render;

/// Export a markdown deck to HTML or PDF.
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
            export_pdf(&rendered.html, file, output)?;
        }
        _ => bail!("Unknown export format: {}", format),
    }

    Ok(())
}

#[cfg(feature = "pdf")]
fn export_pdf(html: &str, file: &Path, output: Option<&Path>) -> Result<()> {
    use headless_chrome::{Browser, LaunchOptions, types::PrintToPdfOptions};
    use std::io::Write;

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| file.with_extension("pdf"));

    // Write HTML to a temp file for Chrome to load
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(html.as_bytes())?;
    let tmp_path = tmp.path().to_path_buf();

    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    let url = format!("file://{}", tmp_path.display());
    tab.navigate_to(&url)?;
    tab.wait_until_navigated()?;

    // Wait for rendering to settle
    std::thread::sleep(std::time::Duration::from_millis(500));

    let pdf_options = PrintToPdfOptions {
        landscape: Some(true),
        print_background: Some(true),
        paper_width: Some(13.333), // 16:9 ratio at ~96dpi
        paper_height: Some(7.5),
        margin_top: Some(0.0),
        margin_bottom: Some(0.0),
        margin_left: Some(0.0),
        margin_right: Some(0.0),
        ..Default::default()
    };

    let pdf_data = tab.print_to_pdf(Some(pdf_options))?;
    std::fs::write(&out_path, &pdf_data)?;

    tracing::info!("Exported PDF to {}", out_path.display());
    Ok(())
}

#[cfg(not(feature = "pdf"))]
fn export_pdf(_html: &str, _file: &Path, _output: Option<&Path>) -> Result<()> {
    bail!("PDF export requires the `pdf` feature flag. Rebuild with: cargo build --features pdf")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_export_html() {
        let mut tmp = tempfile::NamedTempFile::with_suffix(".md").unwrap();
        write!(tmp, "# Test Slide\n\n---\n\n# Slide 2\n").unwrap();

        let out = tempfile::NamedTempFile::with_suffix(".html").unwrap();
        let out_path = out.path().to_path_buf();

        export(tmp.path(), "html", Some(&out_path)).unwrap();

        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("Test Slide"));
    }

    #[test]
    fn test_export_unknown_format() {
        let mut tmp = tempfile::NamedTempFile::with_suffix(".md").unwrap();
        write!(tmp, "# Test\n").unwrap();

        let result = export(tmp.path(), "docx", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown export format"));
    }

    #[cfg(not(feature = "pdf"))]
    #[test]
    fn test_pdf_export_without_feature() {
        let mut tmp = tempfile::NamedTempFile::with_suffix(".md").unwrap();
        write!(tmp, "# Test\n").unwrap();

        let result = export(tmp.path(), "pdf", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pdf"));
    }
}
