use std::path::Path;
use std::process::Command;

/// Opens a file or folder path using the system desktop opener (xdg-open / gio).
pub fn open_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {:?}", path));
    }

    let res = Command::new("xdg-open")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match res {
        Ok(_) => Ok(()),
        Err(_) => {
            // Fallback to gio open if xdg-open fails
            Command::new("gio")
                .args(["open", path.to_str().unwrap_or("")])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to open path: {}", e))
        }
    }
}

/// Opens a file in the configured system editor ($EDITOR or nano).
pub fn open_in_editor(file_path: &Path) -> std::io::Result<std::process::ExitStatus> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    Command::new(&editor).arg(file_path).status()
}

/// Displays a log file in the configured pager viewer ($PAGER or less).
pub fn open_in_pager(log_path: &Path) -> std::io::Result<std::process::ExitStatus> {
    let pager = std::env::var("PAGER")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "less".to_string());

    if pager == "less" {
        Command::new("less").arg("-R").arg(log_path).status()
    } else {
        Command::new(&pager).arg(log_path).status()
    }
}
