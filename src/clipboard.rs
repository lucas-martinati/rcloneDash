use std::io::Write;
use std::process::{Command, Stdio};

/// Copie une chaîne dans le presse-papiers du système Linux.
/// Tente wl-copy (Wayland), xclip/xsel (X11) et la séquence ANSI OSC 52 (terminaux modernes).
pub fn copy_to_clipboard(text: &str) -> bool {
    let mut success = false;

    // 1. Essai wl-copy (Wayland)
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    drop(stdin);
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            success = true;
                        }
                    }
                }
            }
        }
    }

    // 2. Essai xclip (X11)
    if !success {
        if let Ok(mut child) = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    drop(stdin);
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            success = true;
                        }
                    }
                }
            }
        }
    }

    // 3. Essai xsel (alternative X11)
    if !success {
        if let Ok(mut child) = Command::new("xsel")
            .arg("--clipboard")
            .arg("--input")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    drop(stdin);
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            success = true;
                        }
                    }
                }
            }
        }
    }

    // 4. Émission de la séquence ANSI OSC 52 vers stdout pour les émulateurs de terminal
    // Format : \x1b]52;c;<base64-string>\x07
    let b64 = base64_encode(text);
    let osc52 = format!("\x1b]52;c;{}\x07", b64);
    let _ = std::io::stdout().write_all(osc52.as_bytes());
    let _ = std::io::stdout().flush();

    success || !text.is_empty()
}

/// Reads text content from the system clipboard on Linux (Wayland via wl-paste, or X11 via xclip/xsel).
pub fn paste_from_clipboard() -> Option<String> {
    // 1. Try wl-paste (Wayland)
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if let Ok(output) = Command::new("wl-paste").arg("--no-newline").output() {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }

    // 2. Try xclip (X11)
    if let Ok(output) = Command::new("xclip")
        .args(["-selection", "clipboard", "-out"])
        .output()
    {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    // 3. Try xsel (alternative X11)
    if let Ok(output) = Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output()
    {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
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
