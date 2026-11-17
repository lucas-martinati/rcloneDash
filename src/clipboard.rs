#[cfg(not(test))]
use std::io::Write;
#[cfg(not(test))]
use std::process::{Command, Stdio};

#[cfg(not(test))]
fn try_pipe_to_command(prog: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(prog)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(text.as_bytes()).is_ok() {
            drop(stdin);
            if let Ok(status) = child.wait() {
                return status.success();
            }
        }
    }
    false
}

#[cfg(not(test))]
fn try_read_command(prog: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(prog).args(args).output().ok()?;
    if output.status.success() {
        let s = String::from_utf8(output.stdout).ok()?;
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

/// Copie une chaîne dans le presse-papiers du système Linux.
/// Tente wl-copy (Wayland), xclip/xsel (X11) et la séquence ANSI OSC 52 (terminaux modernes).
/// Isolé en `#[cfg(test)]` pour ne jamais écraser le vrai presse-papiers de l'utilisateur durant `cargo test`.
pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(test)]
    {
        !text.is_empty()
    }

    #[cfg(not(test))]
    {
        let mut success = false;

        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            success = try_pipe_to_command("wl-copy", &[], text);
        }
        if !success {
            success = try_pipe_to_command("xclip", &["-selection", "clipboard"], text);
        }
        if !success {
            success = try_pipe_to_command("xsel", &["--clipboard", "--input"], text);
        }

        // Émission de la séquence ANSI OSC 52 vers stdout pour les émulateurs de terminal
        // Format : \x1b]52;c;<base64-string>\x07
        let b64 = base64_encode(text);
        let osc52 = format!("\x1b]52;c;{}\x07", b64);
        let _ = std::io::stdout().write_all(osc52.as_bytes());
        let _ = std::io::stdout().flush();

        success || !text.is_empty()
    }
}

/// Reads text content from the system clipboard on Linux (Wayland via wl-paste, or X11 via xclip/xsel).
pub fn paste_from_clipboard() -> Option<String> {
    #[cfg(test)]
    {
        None
    }

    #[cfg(not(test))]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            if let Some(s) = try_read_command("wl-paste", &["--no-newline"]) {
                return Some(s);
            }
        }
        if let Some(s) = try_read_command("xclip", &["-selection", "clipboard", "-out"]) {
            return Some(s);
        }
        try_read_command("xsel", &["--clipboard", "--output"])
    }
}

pub fn base64_encode(data: &str) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = data.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(CHARSET[(b0 >> 2) as usize] as char);
        out.push(CHARSET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            out.push(CHARSET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(CHARSET[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }
}
