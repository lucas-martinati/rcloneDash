use std::path::Path;
use std::process::Command;

/// Splits a shell-like command variable (e.g. `"code --wait"` or `"less -FRX"`)
/// into `(program, extra_args)`. Falls back to `default_prog` if blank.
pub fn split_command_and_args<'a>(raw: &'a str, default_prog: &'a str) -> (&'a str, Vec<&'a str>) {
    let mut parts = raw.split_whitespace();
    match parts.next() {
        Some(prog) => (prog, parts.collect()),
        None => (default_prog, Vec::new()),
    }
}

/// Opens a file or folder path using the system desktop opener (xdg-open / gio).
pub fn open_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {:?}", path));
    }

    #[cfg(test)]
    {
        let _ = path;
        Ok(())
    }

    #[cfg(not(test))]
    {
        let spawn_detached = |mut cmd: Command| -> Result<(), std::io::Error> {
            let mut child = cmd
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;
            // Reap child asynchronously to avoid leaving <defunct> zombie processes.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            Ok(())
        };

        let mut xdg = Command::new("xdg-open");
        xdg.arg(path);
        if spawn_detached(xdg).is_ok() {
            return Ok(());
        }

        // Fallback to `gio open` if `xdg-open` is unavailable
        let mut gio = Command::new("gio");
        gio.arg("open").arg(path);
        spawn_detached(gio).map_err(|e| format!("Failed to open path: {}", e))
    }
}

/// Opens a file in the configured system editor (`$EDITOR` or `nano`),
/// supporting flags inside `$EDITOR` (e.g. `EDITOR="code --wait"`).
pub fn open_in_editor(file_path: &Path) -> std::io::Result<std::process::ExitStatus> {
    let raw = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let (prog, extra_args) = split_command_and_args(&raw, "nano");
    Command::new(prog).args(extra_args).arg(file_path).status()
}

/// Displays a log file in the configured pager viewer (`$PAGER`, `$EDITOR`, or `less`),
/// supporting flags (e.g. `PAGER="less -FRX"`) and defaulting plain `less` to `less -R`.
pub fn open_in_pager(log_path: &Path) -> std::io::Result<std::process::ExitStatus> {
    let raw = std::env::var("PAGER")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "less".to_string());
    let (prog, extra_args) = split_command_and_args(&raw, "less");

    let mut cmd = Command::new(prog);
    if (prog == "less" || prog.ends_with("/less")) && extra_args.is_empty() {
        cmd.arg("-R");
    } else {
        cmd.args(extra_args);
    }
    cmd.arg(log_path).status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_command_and_args() {
        assert_eq!(
            split_command_and_args("code --wait --new-window", "nano"),
            ("code", vec!["--wait", "--new-window"])
        );
        assert_eq!(split_command_and_args("nvim", "nano"), ("nvim", vec![]));
        assert_eq!(split_command_and_args("   ", "less"), ("less", vec![]));
        assert_eq!(
            split_command_and_args("less -FRX", "less"),
            ("less", vec!["-FRX"])
        );
    }
}
