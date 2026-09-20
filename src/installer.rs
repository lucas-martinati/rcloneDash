use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use crate::config::dirs_home;

/// Checks if rclone is installed in PATH or in ~/.local/bin/rclone, returning its version string if found.
pub fn check_rclone_installed() -> Option<String> {
    // 1. Check in standard PATH
    if let Ok(output) = Command::new("rclone").arg("--version").output() {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Some(first_line) = s.lines().next() {
                    return Some(first_line.trim().to_string());
                }
            }
        }
    }

    // 2. Check ~/.local/bin/rclone directly
    if let Some(home) = dirs_home() {
        let local_bin = home.join(".local/bin/rclone");
        if local_bin.is_file() {
            if let Ok(output) = Command::new(&local_bin).arg("--version").output() {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        if let Some(first_line) = s.lines().next() {
                            return Some(first_line.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Detects the target architecture tag for official rclone release downloads.
pub fn get_arch_tag() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "linux-amd64",
        "aarch64" => "linux-arm64",
        "arm" => "linux-arm-v7",
        "x86" => "linux-386",
        _ => "linux-amd64",
    }
}

/// Downloads and installs rclone into ~/.local/bin/rclone without requiring root privileges.
pub async fn install_rclone_user() -> Result<String, String> {
    let home = dirs_home().ok_or_else(|| "Could not determine user HOME directory".to_string())?;
    let bin_dir = home.join(".local/bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create ~/.local/bin directory: {}", e))?;

    let target_binary = bin_dir.join("rclone");
    let arch_tag = get_arch_tag();
    let download_url = format!("https://downloads.rclone.org/rclone-current-{}.zip", arch_tag);

    let pid = std::process::id();
    let tmp_zip = std::env::temp_dir().join(format!("rclone-install-{}.zip", pid));

    // 1. Download zip archive with curl
    let status = tokio::process::Command::new("curl")
        .args(["-fsSL", "--connect-timeout", "10", "--max-time", "60", "-o"])
        .arg(&tmp_zip)
        .arg(&download_url)
        .status()
        .await
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !status.success() {
        let _ = fs::remove_file(&tmp_zip);
        return Err(format!("Failed to download rclone from {}", download_url));
    }

    // 2. Extract rclone binary using unzip
    let unzip_status = tokio::process::Command::new("unzip")
        .arg("-p")
        .arg(&tmp_zip)
        .arg("*/rclone")
        .output()
        .await;

    let _ = fs::remove_file(&tmp_zip);

    match unzip_status {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => {
            fs::write(&target_binary, out.stdout)
                .map_err(|e| format!("Failed to write binary to {:?}: {}", target_binary, e))?;
            if let Err(e) = fs::set_permissions(&target_binary, fs::Permissions::from_mode(0o755)) {
                eprintln!("Warning: Failed to set 0755 permissions on {:?}: {}", target_binary, e);
            }
        }
        _ => {
            // Fallback: try python zipfile extraction if unzip command is not present
            let py_status = tokio::process::Command::new("python3")
                .args([
                    "-c",
                    &format!(
                        "import zipfile, sys; z = zipfile.ZipFile('{}'); f = next(n for n in z.namelist() if n.endswith('/rclone') or n == 'rclone'); sys.stdout.buffer.write(z.read(f))",
                        tmp_zip.display()
                    ),
                ])
                .output()
                .await;

            if let Ok(py_out) = py_status {
                if py_out.status.success() && !py_out.stdout.is_empty() {
                    fs::write(&target_binary, py_out.stdout)
                        .map_err(|e| format!("Failed to write binary: {}", e))?;
                    if let Err(e) = fs::set_permissions(&target_binary, fs::Permissions::from_mode(0o755)) {
                        eprintln!("Warning: Failed to set 0755 permissions on {:?}: {}", target_binary, e);
                    }
                } else {
                    return Err("Failed to extract rclone binary (neither unzip nor python3 zipfile succeeded)".to_string());
                }
            } else {
                return Err("Failed to extract rclone archive".to_string());
            }
        }
    }

    // 3. Verify installation
    if let Some(ver) = check_rclone_installed() {
        Ok(format!("{} installed successfully at {}", ver, target_binary.display()))
    } else {
        Err("rclone binary was written but could not be executed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_tag() {
        let tag = get_arch_tag();
        assert!(tag.starts_with("linux-"));
    }
}
