use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::term_caps::{Glyphs, TermCaps};

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub asset_size: Option<u64>,
}

pub struct ConsoleStyle {
    pub caps: TermCaps,
}

impl Default for ConsoleStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleStyle {
    pub fn new() -> Self {
        Self {
            caps: TermCaps::detect(),
        }
    }

    pub fn from_caps(caps: TermCaps) -> Self {
        Self { caps }
    }

    /// Color output is allowed (`NO_COLOR` / pipe ⇒ plain text).
    /// Independent from [`TermCaps::live`]: `NO_COLOR` on a real terminal
    /// strips ANSI codes but keeps the live progress bar.
    fn colored(&self) -> bool {
        self.caps.color.supports_ansi()
    }

    pub fn bold(&self, text: &str) -> String {
        if self.colored() { format!("\x1b[1m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn bold_red(&self, text: &str) -> String {
        if self.colored() { format!("\x1b[1;31m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn bold_green(&self, text: &str) -> String {
        if self.colored() { format!("\x1b[1;32m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn bold_cyan(&self, text: &str) -> String {
        if self.colored() { format!("\x1b[1;36m{}\x1b[0m", text) } else { text.to_string() }
    }
    pub fn gray(&self, text: &str) -> String {
        if self.colored() { format!("\x1b[90m{}\x1b[0m", text) } else { text.to_string() }
    }
}

pub fn format_size(bytes: u64) -> String {
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

/// Width of the download progress bar (matches the updater mockup).
pub const PROGRESS_BAR_WIDTH: usize = 30;

/// One wizard step header: `◇  <text>...` (cyan diamond on TTY).
pub fn step_line(style: &ConsoleStyle, text: &str) -> String {
    let g = style.caps.glyphs();
    format!("  {}  {}", style.bold_cyan(g.diamond), text)
}

/// One indented detail line: `│  <text>`.
pub fn detail_line(style: &ConsoleStyle, text: &str) -> String {
    format!("  {}  {}", style.caps.glyphs().vline, text)
}

/// Splits progress into display percentage and bar glyphs.
/// Unknown or empty totals render as `---%` with an empty bar.
/// Unicode variant (see [`progress_parts_in`] for the ASCII fallback).
pub fn progress_parts(downloaded: u64, total: Option<u64>) -> (String, String) {
    progress_parts_in(downloaded, total, &Glyphs::unicode())
}

/// [`progress_parts`] with an explicit glyph set (ASCII on TTY consoles).
pub fn progress_parts_in(downloaded: u64, total: Option<u64>, glyphs: &Glyphs) -> (String, String) {
    match total {
        Some(tot) if tot > 0 => {
            let pct = (downloaded as f64 / tot as f64).clamp(0.0, 1.0);
            let filled = (pct * PROGRESS_BAR_WIDTH as f64).round() as usize;
            let empty = PROGRESS_BAR_WIDTH.saturating_sub(filled);
            (
                format!("{:>3.0}%", pct * 100.0),
                format!(
                    "{}{}",
                    glyphs.bar_fill.repeat(filled),
                    glyphs.bar_empty.repeat(empty)
                ),
            )
        }
        _ => ("---%".to_string(), glyphs.bar_empty.repeat(PROGRESS_BAR_WIDTH)),
    }
}

/// Renders one full progress line (no `\r`, no newline), e.g.
/// `│  [██████░░░░]  42%  1.4 MB / 3.3 MB (1.1 MB/s)`.
///
/// When `done` is true the line is the finished state: a reached total (or
/// an unknown one, including a zero total like `Content-Length: 0`) renders
/// `100%` in green with a full bar, but the total is only displayed when it
/// is actually known and positive — no fabricated ` / ...`.
/// A known-but-unreached total keeps its honest percentage instead.
pub fn progress_line(style: &ConsoleStyle, downloaded: u64, total: Option<u64>, speed: f64, done: bool) -> String {
    // A zero total carries no information (e.g. Content-Length: 0):
    // treat it like an unknown one.
    let glyphs = style.caps.glyphs();
    let unknown_total = total.is_none_or(|t| t == 0);
    let (pct_str, bar) = if done && unknown_total {
        ("100%".to_string(), glyphs.bar_fill.repeat(PROGRESS_BAR_WIDTH))
    } else {
        progress_parts_in(downloaded, total, &glyphs)
    };
    let pct = if pct_str == "100%" {
        style.bold_green(&pct_str)
    } else {
        style.bold(&pct_str)
    };
    let size_info = match total {
        Some(tot) if tot > 0 => format!("{} / {}", format_size(downloaded), format_size(tot)),
        _ => format_size(downloaded),
    };
    let speed_info = format!("{}/s", format_size(speed as u64));

    format!(
        "  {}  {}[{}]{} {}  {} ({})",
        glyphs.vline,
        if style.colored() { "\x1b[32m" } else { "" },
        bar,
        if style.colored() { "\x1b[0m" } else { "" },
        pct,
        style.gray(&size_info),
        style.gray(&speed_info),
    )
}

/// Live-updates the progress bar on the current terminal line.
fn render_progress(style: &ConsoleStyle, downloaded: u64, total: Option<u64>, speed: f64) {
    use std::io::Write;
    print!("\r{}\x1b[K", progress_line(style, downloaded, total, speed, false));
    let _ = std::io::stdout().flush();
}

/// Prints the final progress bar on its own line.
/// An unknown total renders as before (`100%`, full bar, bare size);
/// a known-but-unreached total keeps its honest percentage.
fn finish_progress(style: &ConsoleStyle, downloaded: u64, total: Option<u64>, speed: f64) {
    println!("\r{}\x1b[K", progress_line(style, downloaded, total, speed, true));
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

    let is_system_pkg = std::env::current_exe()
        .map(|p| p.starts_with("/usr/bin"))
        .unwrap_or(false);

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
                let matches_target = if is_system_pkg {
                    name.ends_with(".deb")
                } else {
                    name.ends_with("linux-x86_64.tar.gz")
                };
                if matches_target {
                    download_url = Some(url.to_string());
                    asset_size = size;
                    break;
                } else if name == "rclonedash-linux-x86_64" && download_url.is_none() {
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
pub async fn download_and_install_update(info: &UpdateInfo, caps: TermCaps) -> Result<(), String> {
    let style = ConsoleStyle::from_caps(caps);
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Could not determine executable path: {}", e))?;

    let pid = std::process::id();
    let is_deb = info.download_url.ends_with(".deb");
    let is_archive = info.download_url.ends_with(".tar.gz");
    let tmp_dest = if is_deb {
        std::env::temp_dir().join(format!("rclonedash-update-{}.deb", pid))
    } else if is_archive {
        std::env::temp_dir().join(format!(".rclonedash-update-{}.tar.gz", pid))
    } else {
        std::env::temp_dir().join(format!(".rclonedash-update-{}", pid))
    };

    println!("{}", step_line(&style, &format!("Downloading rcloneDash {}...", style.bold(&format!("v{}", info.latest_version)))));

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

    if style.caps.live {
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

        if style.caps.live && (last_render.elapsed().as_millis() >= 33 || total.is_some_and(|t| downloaded >= t)) {
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

    if style.caps.live {
        let elapsed_secs = start.elapsed().as_secs_f64();
        let speed = if elapsed_secs > 0.0 { downloaded as f64 / elapsed_secs } else { 0.0 };
        finish_progress(&style, downloaded, total, speed);
    } else {
        println!("{}", detail_line(&style, &format!("Downloaded {}", format_size(downloaded))));
    }

    println!("  {}", style.caps.glyphs().vline);

    if is_deb {
        println!("{}", step_line(&style, "Updating system Debian package (/usr/bin/rclonedash)..."));
        let (cmd, args) = if unsafe { libc::geteuid() } == 0 {
            ("apt", vec!["install", "-y", "--reinstall", tmp_dest.to_str().unwrap_or("")])
        } else {
            ("sudo", vec!["apt", "install", "-y", "--reinstall", tmp_dest.to_str().unwrap_or("")])
        };

        let status = tokio::process::Command::new(cmd)
            .args(args)
            .status()
            .await;

        let _ = tokio::fs::remove_file(&tmp_dest).await;

        match status {
            Ok(st) if st.success() => {
                println!("{}", detail_line(&style, "Debian package updated successfully"));
                Ok(())
            }
            _ => Err(format!(
                "Failed to update Debian package with sudo apt.\nTo update manually, run:\n    sudo apt install --reinstall {:?}",
                tmp_dest
            )),
        }
    } else if is_archive {
        let extract_dir = std::env::temp_dir().join(format!(".rclonedash-update-{}", pid));
        let _ = tokio::fs::create_dir_all(&extract_dir).await;

        println!("{}", step_line(&style, "Extracting release package..."));
        let untar = tokio::process::Command::new("tar")
            .arg("-xzf")
            .arg(&tmp_dest)
            .arg("--strip-components=1")
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .await;
        let _ = std::fs::remove_file(&tmp_dest);

        match untar {
            Ok(st) if st.success() => {
                let installer = extract_dir.join("install.sh");
                if installer.is_file() {
                    println!("{}", step_line(&style, "Updating binary, systemd services, desktop entry, and icon..."));
                    let inst = tokio::process::Command::new("bash")
                        .arg(&installer)
                        .current_dir(&extract_dir)
                        .status()
                        .await;
                    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
                    match inst {
                        Ok(ist) if ist.success() => {
                            println!("{}", detail_line(&style, "Updated all components successfully (user configuration preserved)"));
                            Ok(())
                        }
                        _ => Err("Installer script failed during update".to_string()),
                    }
                } else {
                    let new_bin = extract_dir.join("rclonedash");
                    let res = if new_bin.is_file() {
                        std::fs::rename(&new_bin, &current_exe)
                            .map_err(|e| format!("Failed to replace binary: {}", e))
                    } else {
                        Err("Archive did not contain rclonedash binary".to_string())
                    };
                    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
                    res
                }
            }
            _ => {
                let _ = tokio::fs::remove_dir_all(&extract_dir).await;
                Err("Failed to extract update archive".to_string())
            }
        }
    } else {
        println!("{}", step_line(&style, &format!("Installing binary to {}...", style.bold(&current_exe.display().to_string()))));

        // Set executable permissions (0755)
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&tmp_dest, perms) {
            let _ = std::fs::remove_file(&tmp_dest);
            return Err(format!("Failed to set permissions: {}", e));
        }
        println!("{}", detail_line(&style, "Applied executable permissions (0755)"));

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
        println!("{}", detail_line(&style, "Replaced binary atomically"));
        Ok(())
    }
}

/// Computes the demo "latest" version: current patch + 1.
/// Falls back to `<current>+demo1` when the current version is not semver.
pub fn demo_latest_version(current: &str) -> String {
    match parse_semver(current) {
        Some((maj, min, patch)) => format!("{}.{}.{}", maj, min, patch + 1),
        None => format!("{}+demo1", current.trim()),
    }
}

/// Simulates the full update flow on screen without any network access or
/// filesystem side effect. Visual test entry point (`rclonedash --update-demo`).
pub async fn demo_update_flow(caps: TermCaps) {
    let style = ConsoleStyle::from_caps(caps);
    let current = env!("CARGO_PKG_VERSION");
    let latest = demo_latest_version(current);
    let g = style.caps.glyphs();

    println!("  {}{} {}", g.corner_tl, g.hline, style.bold_red("rcloneDash Updater (demo)"));
    println!("  {}", g.vline);
    println!("{}", step_line(&style, "Checking for latest release..."));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    println!(
        "  {}  Found {} (current: {})",
        g.vline,
        style.bold(&format!("v{}", latest)),
        style.gray(&format!("v{}", current))
    );
    println!("  {}", g.vline);
    println!(
        "{}",
        step_line(&style, &format!("Downloading rcloneDash {}...", style.bold(&format!("v{}", latest))))
    );

    // Fake ~1.4 MB payload streamed at ~1.1 MB/s, like a real release asset.
    let total: u64 = 1_478_796;
    let start = std::time::Instant::now();
    if style.caps.live {
        let chunk: u64 = 46_080;
        let mut downloaded: u64 = 0;
        render_progress(&style, 0, Some(total), 0.0);
        while downloaded < total {
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            downloaded = (downloaded + chunk).min(total);
            let speed = downloaded as f64 / start.elapsed().as_secs_f64().max(0.001);
            render_progress(&style, downloaded, Some(total), speed);
        }
        let speed = downloaded as f64 / start.elapsed().as_secs_f64().max(0.001);
        finish_progress(&style, downloaded, Some(total), speed);
    } else {
        // No carriage-return animation off-TTY: single honest final line.
        println!("{}", progress_line(&style, total, Some(total), 1_152_000.0, true));
    }

    println!("  {}", g.vline);
    println!("{}", step_line(&style, "Extracting release package..."));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    println!("{}", step_line(&style, "Updating binary, systemd services, desktop entry, and icon..."));
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    println!("  {}", g.vline);
    println!("  {}{} {}", g.corner_bl, g.hline, style.bold_green(&format!("{} Successfully updated to v{}!", g.spark, latest)));
    println!("{}", detail_line(&style, &style.gray("demo mode — nothing was downloaded or installed")));
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

    #[test]
    fn test_format_size_boundaries() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1_478_796), "1.4 MB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }

    #[test]
    fn test_progress_parts() {
        // Empty bar at start
        let (pct, bar) = progress_parts(0, Some(100));
        assert_eq!(pct, "  0%");
        assert_eq!(bar, "░".repeat(PROGRESS_BAR_WIDTH));

        // Halfway: bar split evenly
        let (pct, bar) = progress_parts(50, Some(100));
        assert_eq!(pct, " 50%");
        assert_eq!(bar, format!("{}{}", "█".repeat(15), "░".repeat(15)));

        // Complete and over-complete clamp to a full bar
        let (pct, bar) = progress_parts(100, Some(100));
        assert_eq!(pct, "100%");
        assert_eq!(bar, "█".repeat(PROGRESS_BAR_WIDTH));
        let (pct, bar) = progress_parts(200, Some(100));
        assert_eq!(pct, "100%");
        assert_eq!(bar, "█".repeat(PROGRESS_BAR_WIDTH));

        // Unknown or empty totals render honestly, never as done
        let (pct, bar) = progress_parts(5, None);
        assert_eq!(pct, "---%");
        assert_eq!(bar, "░".repeat(PROGRESS_BAR_WIDTH));
        let (pct, bar) = progress_parts(5, Some(0));
        assert_eq!(pct, "---%");
        assert_eq!(bar, "░".repeat(PROGRESS_BAR_WIDTH));
    }

    #[test]
    fn test_progress_line_plain_matches_mockup() {
        // Off-TTY style renders deterministically, without ANSI codes.
        let style = ConsoleStyle::from_caps(TermCaps::plain());
        let line = progress_line(&style, 1024, Some(2048), 1024.0, false);
        let half = "█".repeat(15);
        let half_empty = "░".repeat(15);
        let expected = format!("  │  [{half}{half_empty}]  50%  1.0 KB / 2.0 KB (1.0 KB/s)");
        assert_eq!(line, expected);
    }

    #[test]
    fn test_progress_line_done_without_known_total() {
        // Finished with unknown total: 100% and full bar like before, but
        // the size stands alone — no fabricated " / ...".
        let style = ConsoleStyle::from_caps(TermCaps::plain());
        let full = "█".repeat(PROGRESS_BAR_WIDTH);
        let line = progress_line(&style, 1_478_796, None, 1_152_000.0, true);
        let expected = format!("  │  [{full}] 100%  1.4 MB (1.1 MB/s)");
        assert_eq!(line, expected);
        assert!(!line.contains(" / "));

        // A zero total (e.g. Content-Length: 0) behaves like unknown.
        let line = progress_line(&style, 1_478_796, Some(0), 1_152_000.0, true);
        assert_eq!(line, expected);

        // ... but live (not done) with zero total stays honest.
        let (pct, _) = progress_parts(1_478_796, Some(0));
        assert_eq!(pct, "---%");
    }

    #[test]
    fn test_progress_line_done_with_unreached_total_stays_honest() {
        // Known-but-unreached total at the end: real percentage, not forced.
        let style = ConsoleStyle::from_caps(TermCaps::plain());
        let line = progress_line(&style, 512, Some(2048), 512.0, true);
        let filled = "█".repeat(8);
        let empty = "░".repeat(22);
        let expected = format!("  │  [{filled}{empty}]  25%  512 B / 2.0 KB (512 B/s)");
        assert_eq!(line, expected);
    }

    #[test]
    fn test_progress_line_tty_colors() {
        // On-TTY style wraps bar, percentage and sizes in ANSI codes.
        let style = ConsoleStyle::from_caps(TermCaps::full());
        let half = "█".repeat(15);
        let half_empty = "░".repeat(15);
        let line = progress_line(&style, 1024, Some(2048), 1024.0, false);
        let expected = format!(
            "  │  \x1b[32m[{half}{half_empty}]\x1b[0m \x1b[1m 50%\x1b[0m  \x1b[90m1.0 KB / 2.0 KB\x1b[0m (\x1b[90m1.0 KB/s\x1b[0m)"
        );
        assert_eq!(line, expected);

        // Finished, unknown total: green 100% with bare size.
        let full = "█".repeat(PROGRESS_BAR_WIDTH);
        let line = progress_line(&style, 1_478_796, None, 1_152_000.0, true);
        let expected = format!(
            "  │  \x1b[32m[{full}]\x1b[0m \x1b[1;32m100%\x1b[0m  \x1b[90m1.4 MB\x1b[0m (\x1b[90m1.1 MB/s\x1b[0m)"
        );
        assert_eq!(line, expected);
    }

    #[test]
    fn test_no_color_strips_ansi_but_keeps_live_bar() {
        // NO_COLOR on a real terminal: no escape codes, but the live
        // animation flag stays on (color and animation are independent).
        let caps = TermCaps {
            live: true,
            ..TermCaps::plain()
        };
        let style = ConsoleStyle::from_caps(caps);
        assert!(style.caps.live);
        let half = "█".repeat(15);
        let half_empty = "░".repeat(15);
        let line = progress_line(&style, 1024, Some(2048), 1024.0, false);
        assert!(!line.contains('\x1b'));
        let expected = format!("  │  [{half}{half_empty}]  50%  1.0 KB / 2.0 KB (1.0 KB/s)");
        assert_eq!(line, expected);
    }

    #[test]
    fn test_step_and_detail_lines() {
        let style = ConsoleStyle::from_caps(TermCaps::plain());
        assert_eq!(step_line(&style, "Checking for latest release..."), "  ◇  Checking for latest release...");
        assert_eq!(detail_line(&style, "hello"), "  │  hello");
    }

    #[test]
    fn test_ascii_caps_use_fallback_glyphs_with_same_width() {
        use unicode_width::UnicodeWidthStr;
        let ascii_caps = TermCaps {
            ascii: true,
            ..TermCaps::plain()
        };
        let style = ConsoleStyle::from_caps(ascii_caps);
        assert_eq!(step_line(&style, "Checking..."), "  o  Checking...");
        assert_eq!(detail_line(&style, "hello"), "  |  hello");

        let half = "#".repeat(15);
        let half_empty = "-".repeat(15);
        let line = progress_line(&style, 1024, Some(2048), 1024.0, false);
        let expected = format!("  |  [{half}{half_empty}]  50%  1.0 KB / 2.0 KB (1.0 KB/s)");
        assert_eq!(line, expected);

        // Same display width as the Unicode variant: alignment is preserved.
        let uni = ConsoleStyle::from_caps(TermCaps::plain());
        let uni_line = progress_line(&uni, 1024, Some(2048), 1024.0, false);
        assert_eq!(line.width(), uni_line.width());
    }

    #[test]
    fn test_demo_latest_version() {
        assert_eq!(demo_latest_version("1.0.32"), "1.0.33");
        assert_eq!(demo_latest_version("v2.10.4-beta1"), "2.10.5");
        assert_eq!(demo_latest_version("nonsense"), "nonsense+demo1");
    }
}
