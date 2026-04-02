use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Update (or reinstall) the slides binary from GitHub releases.
pub fn self_update(force: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Fetch latest release info from GitHub
    let api_output = OsCommand::new("curl")
        .args([
            "-fsSL",
            "https://api.github.com/repos/inhesrom/slides/releases/latest",
        ])
        .output()
        .context("failed to run curl — is it installed?")?;
    if !api_output.status.success() {
        return Err(anyhow!(
            "failed to fetch latest release info from GitHub (curl exit {})",
            api_output.status
        ));
    }
    let api_body = String::from_utf8_lossy(&api_output.stdout);

    let tag = parse_latest_release_tag(&api_body)?;
    let latest_version = tag.strip_prefix('v').unwrap_or(&tag);

    if latest_version == current_version && !force {
        println!("slides is already up to date (v{current_version})");
        return Ok(());
    }

    if latest_version == current_version {
        println!("reinstalling slides v{current_version}...");
    } else {
        println!("updating slides v{current_version} -> v{latest_version}...");
    }

    // Detect platform
    let os_output = OsCommand::new("uname").arg("-s").output()?;
    let os_name = String::from_utf8_lossy(&os_output.stdout)
        .trim()
        .to_lowercase();

    let arch_output = OsCommand::new("uname").arg("-m").output()?;
    let arch_name = String::from_utf8_lossy(&arch_output.stdout)
        .trim()
        .to_string();

    let target = detect_release_target(&os_name, &arch_name)?;

    let url = format!(
        "https://github.com/inhesrom/slides/releases/download/{tag}/slides-{target}.tar.gz"
    );

    // Download to a temp directory
    let tmp_dir = std::env::temp_dir().join(format!("slides-update-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir)?;
    let _cleanup = TempDirGuard(tmp_dir.clone());

    let tarball = tmp_dir.join("slides.tar.gz");
    let dl_status = OsCommand::new("curl")
        .args(["-fsSL", &url, "-o"])
        .arg(&tarball)
        .status()
        .context("failed to run curl for download")?;
    if !dl_status.success() {
        return Err(anyhow!("failed to download release tarball from {url}"));
    }

    // Extract
    let extract_status = OsCommand::new("tar")
        .arg("xzf")
        .arg(&tarball)
        .arg("-C")
        .arg(&tmp_dir)
        .status()
        .context("failed to run tar")?;
    if !extract_status.success() {
        return Err(anyhow!("failed to extract release tarball"));
    }

    // Replace the current binary
    let current_exe =
        std::env::current_exe().context("cannot determine current executable path")?;
    let new_binary = tmp_dir.join("slides");
    if !new_binary.exists() {
        return Err(anyhow!(
            "extracted archive does not contain 'slides' binary"
        ));
    }

    let downloaded_version = read_slides_version(&new_binary).with_context(|| {
        format!(
            "downloaded release asset at {} is not a valid slides binary",
            new_binary.display()
        )
    })?;
    if downloaded_version != latest_version {
        return Err(anyhow!(
            "downloaded release asset reports v{downloaded_version}, expected v{latest_version}. The GitHub release may contain a stale binary."
        ));
    }

    // Remove the running binary first — Linux allows unlinking an in-use
    // executable but blocks writing to it (ETXTBSY / "Text file busy").
    std::fs::remove_file(&current_exe).with_context(|| {
        format!(
            "failed to remove old binary at {}. You may need to run with sudo.",
            current_exe.display()
        )
    })?;
    std::fs::copy(&new_binary, &current_exe)
        .with_context(|| format!("failed to install new binary at {}.", current_exe.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    // Verify the installed binary
    let installed_version = read_slides_version(&current_exe).with_context(|| {
        format!(
            "installed binary at {} could not be verified after update",
            current_exe.display()
        )
    })?;
    if installed_version != latest_version {
        return Err(anyhow!(
            "updated binary at {} still reports v{}, expected v{}",
            current_exe.display(),
            installed_version,
            latest_version
        ));
    }

    // Check if there's a different slides on PATH
    if let Some(path_binary) = find_binary_on_path("slides") {
        if !same_executable(&path_binary, &current_exe) {
            let path_version = read_slides_version(&path_binary).with_context(|| {
                format!(
                    "`slides` on PATH resolves to {}, which is different from the updated binary at {}",
                    path_binary.display(),
                    current_exe.display()
                )
            })?;
            if path_version != latest_version {
                return Err(anyhow!(
                    "updated {} to v{}, but `slides` on PATH resolves to {} and reports v{}. Adjust PATH or update that install.",
                    current_exe.display(),
                    latest_version,
                    path_binary.display(),
                    path_version
                ));
            }
        }
    }

    println!(
        "slides updated to v{latest_version} at {}",
        current_exe.display()
    );
    Ok(())
}

fn parse_latest_release_tag(api_body: &str) -> Result<String> {
    let release: GitHubRelease =
        serde_json::from_str(api_body).context("failed to parse GitHub release response")?;
    let tag = release.tag_name.trim();
    if tag.is_empty() {
        return Err(anyhow!("GitHub release response did not include tag_name"));
    }
    Ok(tag.to_string())
}

fn detect_release_target(os_name: &str, arch_name: &str) -> Result<&'static str> {
    match (os_name, arch_name) {
        ("darwin", "arm64" | "aarch64") => Ok("aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        _ => Err(anyhow!("unsupported platform: {os_name} {arch_name}")),
    }
}

fn parse_slides_version_output(output: &str) -> Option<&str> {
    let line = output.lines().find(|line| !line.trim().is_empty())?.trim();
    let version = line.strip_prefix("slides ")?;
    Some(version.strip_prefix('v').unwrap_or(version))
}

fn read_slides_version(path: &Path) -> Result<String> {
    let output = OsCommand::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to run {} --version", path.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "{} --version exited with {}",
            path.display(),
            output.status
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_slides_version_output(&stdout)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            anyhow!(
                "unexpected version output from {}: {}",
                path.display(),
                stdout.trim()
            )
        })
}

fn find_binary_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn same_executable(lhs: &Path, rhs: &Path) -> bool {
    if lhs == rhs {
        return true;
    }
    match (lhs.canonicalize(), rhs.canonicalize()) {
        (Ok(lhs), Ok(rhs)) => lhs == rhs,
        _ => false,
    }
}

struct TempDirGuard(PathBuf);
impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_latest_release_tag() {
        let body = r#"{"tag_name": "v0.1.1", "name": "v0.1.1"}"#;
        let tag = parse_latest_release_tag(body).unwrap();
        assert_eq!(tag, "v0.1.1");
    }

    #[test]
    fn test_parse_latest_release_tag_empty() {
        let body = r#"{"tag_name": "", "name": ""}"#;
        assert!(parse_latest_release_tag(body).is_err());
    }

    #[test]
    fn test_detect_release_target_macos() {
        assert_eq!(
            detect_release_target("darwin", "arm64").unwrap(),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            detect_release_target("darwin", "aarch64").unwrap(),
            "aarch64-apple-darwin"
        );
    }

    #[test]
    fn test_detect_release_target_linux() {
        assert_eq!(
            detect_release_target("linux", "x86_64").unwrap(),
            "x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn test_detect_release_target_unsupported() {
        assert!(detect_release_target("windows", "x86_64").is_err());
        assert!(detect_release_target("linux", "armv7").is_err());
    }

    #[test]
    fn test_parse_slides_version_output() {
        assert_eq!(
            parse_slides_version_output("slides 0.1.1\n"),
            Some("0.1.1")
        );
        assert_eq!(
            parse_slides_version_output("slides v0.1.1\n"),
            Some("0.1.1")
        );
    }

    #[test]
    fn test_parse_slides_version_output_invalid() {
        assert_eq!(parse_slides_version_output("unknown"), None);
        assert_eq!(parse_slides_version_output(""), None);
    }
}
