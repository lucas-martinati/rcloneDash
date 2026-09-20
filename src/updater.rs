use std::path::Path;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
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

    // Identify best download asset URL
    let mut download_url = None;
    if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            if let (Some(name), Some(url)) = (
                asset.get("name").and_then(|v| v.as_str()),
                asset.get("browser_download_url").and_then(|v| v.as_str())
            ) {
                if name == "rclonedash-linux-x86_64" {
                    download_url = Some(url.to_string());
                    break;
                } else if name.ends_with("linux-x86_64.tar.gz") && download_url.is_none() {
                    download_url = Some(url.to_string());
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
    }))
}

/// Downloads and atomically replaces the current binary with the latest release
pub async fn download_and_install_update(info: &UpdateInfo) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Could not determine executable path: {}", e))?;

    let exe_dir = current_exe.parent()
        .unwrap_or_else(|| Path::new("."));

    let pid = std::process::id();
    let tmp_dest = exe_dir.join(format!(".rclonedash-update-{}", pid));

    if info.download_url.ends_with(".tar.gz") {
        let tar_tmp = exe_dir.join(format!(".rclonedash-update-{}.tar.gz", pid));
        let status = tokio::process::Command::new("curl")
            .args(["-fSL", "--progress-bar", "-o", tar_tmp.to_str().unwrap(), &info.download_url])
            .status()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !status.success() {
            let _ = std::fs::remove_file(&tar_tmp);
            return Err("Download failed (archive corrupted or connection lost)".to_string());
        }

        let untar = tokio::process::Command::new("tar")
            .args(["-xzf", tar_tmp.to_str().unwrap(), "--strip-components=1", "-C", exe_dir.to_str().unwrap()])
            .status()
            .await;
        let _ = std::fs::remove_file(&tar_tmp);

        match untar {
            Ok(st) if st.success() => Ok(()),
            _ => Err("Failed to extract update archive".to_string()),
        }
    } else {
        // Direct standalone binary download
        let status = tokio::process::Command::new("curl")
            .args(["-fSL", "--progress-bar", "-o", tmp_dest.to_str().unwrap(), &info.download_url])
            .status()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !status.success() {
            let _ = std::fs::remove_file(&tmp_dest);
            return Err("Download failed (connection lost or asset not found)".to_string());
        }

        // Set executable permissions (0755)
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&tmp_dest, perms) {
            let _ = std::fs::remove_file(&tmp_dest);
            return Err(format!("Failed to set permissions: {}", e));
        }

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
