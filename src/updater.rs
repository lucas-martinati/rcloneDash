use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub asset_size: Option<u64>,
}

pub struct ConsoleStyle {
    pub is_tty: bool,
}

impl Default for ConsoleStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleStyle {
    pub fn new() -> Self {
        use std::io::IsTerminal;
        Self {
            is_tty: std::io::stdout().is_terminal(),
        }
    }

    pub fn bold(&self, text: &str) -> String {
        if self.is_tty { format!("\x1b[1m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn bold_red(&self, text: &str) -> String {
        if self.is_tty { format!("\x1b[1;31m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn bold_green(&self, text: &str) -> String {
        if self.is_tty { format!("\x1b[1;32m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn bold_cyan(&self, text: &str) -> String {
        if self.is_tty { format!("\x1b[1;36m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn gray(&self, text: &str) -> String {
        if self.is_tty { format!("\x1b[90m{}\x1b[0m", text) } else { text.to_string() }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn render_progress(style: &ConsoleStyle, downloaded: u64, total: Option<u64>, speed: f64) {
    use std::io::Write;
    let bar_width: usize = 30;
    let (pct_str, bar) = if let Some(tot) = total {
        if tot > 0 {
            let pct = (downloaded as f64 / tot as f64).clamp(0.0, 1.0);
            let filled = (pct * bar_width as f64).round() as usize;
            let empty = bar_width.saturating_sub(filled);
            (
                format!("{:>3.0}%", pct * 100.0),
                format!("{}{}", "█".repeat(filled), "░".repeat(empty)),
            )
        } else {
            ("---%".to_string(), "░".repeat(bar_width))
        }
    } else {
        ("---%".to_string(), "░".repeat(bar_width))
    };

    let size_info = if let Some(tot) = total {
        format!("{} / {}", format_size(downloaded), format_size(tot))
    } else {
        format_size(downloaded)
    };
    let speed_info = format!("{}/s", format_size(speed as u64));

    print!(
        "\r  │  {}[{}]{} {}  {} ({})\x1b[K",
        if style.is_tty { "\x1b[32m" } else { "" },
        bar,
        if style.is_tty { "\x1b[0m" } else { "" },
        style.bold(&pct_str),
        style.gray(&size_info),
        style.gray(&speed_info),
    );
    let _ = std::io::stdout().flush();
}

fn finish_progress(style: &ConsoleStyle, downloaded: u64, total: Option<u64>, speed: f64) {
    use std::io::Write;
    let bar_width: usize = 30;
    let bar = "█".repeat(bar_width);
    let size_info = if let Some(tot) = total {
        format!("{} / {}", format_size(downloaded), format_size(tot))
    } else {
        format_size(downloaded)
    };
    let speed_info = format!("{}/s", format_size(speed as u64));

    println!(
        "\r  │  {}[{}]{} {}  {} ({})\x1b[K",
        if style.is_tty { "\x1b[32m" } else { "" },
        bar,
        if style.is_tty { "\x1b[0m" } else { "" },
        style.bold_green("100%"),
        style.gray(&size_info),
        style.gray(&speed_info),
    );
    let _ = std::io::stdout().flush();
}

/// Parses a semantic version string (e.g. "1.0.0", "v1.0.1", "1.2.3-beta") into (major, minor, patch)
pub fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let clean = s.trim().trim_start_matches('v').trim_start_matches('V');
    let core = clean.split('-').next().unwrap_or(clean);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Returns true if `remote` version is strictly newer than `current` version
pub fn is_newer_version(remote: &str, current: &str) -> bool {
    if let (Some(r), Some(c)) = (parse_semver(remote), parse_semver(current)) {
        r > c
    } else {
        false
    }
}

/// Queries the GitHub Releases API for the latest published release of rcloneDash
pub async fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    let current = env!("CARGO_PKG_VERSION");
    let output = tokio::process::Command::new("curl")
        .args([
            "-fsSL",
            "-H", "User-Agent: rclonedash-updater",
            "-H", "Accept: application/vnd.github.v3+json",
            "--connect-timeout", "4",
            "--max-time", "8",
            "https://api.github.com/repos/lucas-martinati/rcloneDash/releases/latest",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to execute curl: {}", e))?;

    if !output.status.success() {
        return Err(format!("GitHub API request failed (exit code: {:?})", output.status.code()));
    }

    let body = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in GitHub response: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse release JSON: {}", e))?;

    let tag_name = json.get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing tag_name in release response".to_string())?;

    let remote_ver = tag_name.trim_start_matches('v').trim_start_matches('V');

    if !is_newer_version(remote_ver, current) {
        return Ok(None);
    }

    // Identify best download asset URL and its size
    let mut download_url = None;
    let mut asset_size = None;
    if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            if let (Some(name), Some(url)) = (
                asset.get("name").and_then(|v| v.as_str()),
                asset.get("browser_download_url").and_then(|v| v.as_str())
            ) {
                let size = asset.get("size").and_then(|v| v.as_u64());
                if name == "rclonedash-linux-x86_64" {
                    download_url = Some(url.to_string());
                    asset_size = size;
                    break;
                } else if name.ends_with("linux-x86_64.tar.gz") && download_url.is_none() {
                    download_url = Some(url.to_string());
                    asset_size = size;
                }
            }
        }
    }

    let dl_url = download_url.unwrap_or_else(|| {
        "https://github.com/lucas-martinati/rcloneDash/releases/latest/download/rclonedash-linux-x86_64".to_string()
    });

    Ok(Some(UpdateInfo {
        current_version: current.to_string(),
        latest_version: remote_ver.to_string(),
        download_url: dl_url,
        asset_size,
    }))
}

/// Downloads and atomically replaces the current binary with the latest release
pub async fn download_and_install_update(info: &UpdateInfo) -> Result<(), String> {
    let style = ConsoleStyle::new();
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Could not determine executable path: {}", e))?;

    let exe_dir = current_exe.parent()
        .unwrap_or_else(|| Path::new("."));

    let pid = std::process::id();
    let is_archive = info.download_url.ends_with(".tar.gz");
    let tmp_dest = if is_archive {
        exe_dir.join(format!(".rclonedash-update-{}.tar.gz", pid))
    } else {
        exe_dir.join(format!(".rclonedash-update-{}", pid))
    };

    println!("  {}  Downloading rcloneDash {}...", style.bold_cyan("◇"), style.bold(&format!("v{}", info.latest_version)));

    let mut child = tokio::process::Command::new("curl")
        .args(["-fsSL", &info.download_url])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to execute curl: {}", e))?;

    let mut stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture download stream".to_string())?;

    let mut file = tokio::fs::File::create(&tmp_dest).await
        .map_err(|e| format!("Could not create temporary file {:?}: {}", tmp_dest, e))?;

    let mut buffer = [0u8; 16384];
    let mut downloaded: u64 = 0;
    let total = info.asset_size;
    let start = std::time::Instant::now();
    let mut last_render = std::time::Instant::now();

    if style.is_tty {
        render_progress(&style, 0, total, 0.0);
    }

    loop {
        let n = stdout.read(&mut buffer).await
            .map_err(|e| format!("Download stream read error: {}", e))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).await
            .map_err(|e| format!("File write error: {}", e))?;
        downloaded += n as u64;

        if style.is_tty && (last_render.elapsed().as_millis() >= 33 || total.is_some_and(|t| downloaded >= t)) {
            let elapsed_secs = start.elapsed().as_secs_f64();
            let speed = if elapsed_secs > 0.0 { downloaded as f64 / elapsed_secs } else { 0.0 };
            render_progress(&style, downloaded, total, speed);
            last_render = std::time::Instant::now();
        }
    }

    file.flush().await.map_err(|e| format!("File flush error: {}", e))?;
    drop(file);

    let status = child.wait().await
        .map_err(|e| format!("curl process error: {}", e))?;
    if !status.success() {
        let _ = tokio::fs::remove_file(&tmp_dest).await;
        return Err("Download failed (connection lost or asset not found)".to_string());
    }

    if style.is_tty {
        let elapsed_secs = start.elapsed().as_secs_f64();
        let speed = if elapsed_secs > 0.0 { downloaded as f64 / elapsed_secs } else { 0.0 };
        finish_progress(&style, downloaded, total, speed);
    } else {
        println!("  │  Downloaded {}", format_size(downloaded));
    }

    println!("  │");

    if is_archive {
        println!("  {}  Extracting archive to {}...", style.bold_cyan("◇"), style.bold(&exe_dir.display().to_string()));
        let untar = tokio::process::Command::new("tar")
            .arg("-xzf")
            .arg(&tmp_dest)
            .arg("--strip-components=1")
            .arg("-C")
            .arg(exe_dir)
            .status()
            .await;
        let _ = std::fs::remove_file(&tmp_dest);

        match untar {
            Ok(st) if st.success() => {
                println!("  │  Extracted and updated files successfully");
                Ok(())
            }
            _ => Err("Failed to extract update archive".to_string()),
        }
    } else {
        println!("  {}  Installing binary to {}...", style.bold_cyan("◇"), style.bold(&current_exe.display().to_string()));

        // Set executable permissions (0755)
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&tmp_dest, perms) {
            let _ = std::fs::remove_file(&tmp_dest);
            return Err(format!("Failed to set permissions: {}", e));
        }
        println!("  │  Applied executable permissions (0755)");

        // Atomic replace
        if let Err(e) = std::fs::rename(&tmp_dest, &current_exe) {
            let _ = std::fs::remove_file(&tmp_dest);
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                return Err(format!(
                    "Permission denied when updating {:?}.\nIf installed system-wide in /usr/bin, run with sudo:\n    sudo rclonedash --update",
                    current_exe
                ));
            }
            return Err(format!("Failed to replace executable: {}", e));
        }
        println!("  │  Replaced binary atomically");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver() {
        assert_eq!(parse_semver("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_semver("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("V2.10.4-beta1"), Some((2, 10, 4)));
        assert_eq!(parse_semver("invalid"), None);
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("v1.1.0", "1.0.9"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
        assert!(!is_newer_version("invalid", "1.0.0"));
    }
}
