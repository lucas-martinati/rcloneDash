use std::fs;

use crate::config::rclone_config_file;

/// Informations sur un remote rclone déclaré dans `rclone.conf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInfo {
    /// Nom de section tel que déclaré (sans `:` final).
    pub name: String,
    /// Valeur de `type = ...` dans la section (minuscules), ex. `drive`, `s3`, `dropbox`.
    /// `None` si la section ne déclare pas de type.
    pub kind: Option<String>,
}

impl RemoteInfo {
    /// `true` si ce remote est un Google Drive (`type = drive`).
    pub fn is_drive(&self) -> bool {
        self.kind.as_deref().map(|k| k == "drive").unwrap_or(false)
    }

    /// Nom d'affichage avec le `:` final attendu par rclone (`gdrive:`).
    pub fn display_name(&self) -> String {
        if self.name.ends_with(':') {
            self.name.clone()
        } else {
            format!("{}:", self.name)
        }
    }

    /// Libellé humain court, ex. `Google Drive (drive)` ou `MonS3 (s3)`.
    pub fn human_label(&self) -> String {
        match self.kind.as_deref() {
            Some("drive") => format!("{} (Google Drive)", self.display_name()),
            Some(k) => format!("{} ({})", self.display_name(), k),
            None => self.display_name(),
        }
    }
}

/// Parse le contenu brut d'un `rclone.conf` et renvoie les remotes détectés.
pub fn parse_remotes(content: &str) -> Vec<RemoteInfo> {
    let mut remotes = Vec::new();
    let mut current: Option<RemoteInfo> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if let Some(r) = current.take() {
                remotes.push(r);
            }
            let name = trimmed[1..trimmed.len() - 1].trim().to_string();
            if !name.is_empty() {
                current = Some(RemoteInfo { name, kind: None });
            }
            continue;
        }
        if let Some(ref mut r) = current {
            if r.kind.is_none() {
                if let Some((key, val)) = trimmed.split_once('=') {
                    if key.trim().eq_ignore_ascii_case("type") {
                        let kind = val.trim().to_lowercase();
                        if !kind.is_empty() {
                            r.kind = Some(kind);
                        }
                    }
                }
            }
        }
    }
    if let Some(r) = current.take() {
        remotes.push(r);
    }
    remotes
}

/// Liste les remotes configurés dans `rclone.conf` (vide si absent/illisible).
pub fn list_remotes() -> Vec<RemoteInfo> {
    let path = rclone_config_file();
    match fs::read_to_string(&path) {
        Ok(content) => parse_remotes(&content),
        Err(_) => Vec::new(),
    }
}

/// Type (`drive`, `s3`, ...) du remote configuré, ou `None` si inconnu.
pub fn remote_type(remote: &str) -> Option<String> {
    let target = remote.trim_end_matches(':').trim().to_lowercase();
    if target.is_empty() {
        return None;
    }
    list_remotes()
        .into_iter()
        .find(|r| r.name.to_lowercase() == target)
        .and_then(|r| r.kind)
}

/// `true` si le remote configuré est un Google Drive (`type = drive`).
///
/// Un remote inconnu (pas de section dans `rclone.conf`) retourne `false` :
/// les options spécifiques à Drive sont alors masquées au lieu d'être
/// proposées à tort.
pub fn is_drive_remote(remote: &str) -> bool {
    remote_type(remote).as_deref() == Some("drive")
}

/// Le remote configuré existe-t-il dans `rclone.conf` ?
pub fn remote_exists(remote: &str) -> bool {
    let target = remote.trim_end_matches(':').trim().to_lowercase();
    if target.is_empty() {
        return false;
    }
    list_remotes()
        .iter()
        .any(|r| r.name.to_lowercase() == target)
}

/// Nom convivial du remote pour l'affichage, ex. `GoogleDrive:` ou `MonS3 (s3):`.
///
/// Ne fait aucune supposition Google : le type réel est lu depuis `rclone.conf`.
pub fn describe_remote(remote: &str) -> String {
    let trimmed = remote.trim();
    if trimmed.is_empty() {
        return "(no remote configured)".to_string();
    }
    match remote_type(trimmed) {
        Some(k) if k == "drive" => trimmed.to_string(),
        Some(k) => format!("{} [{}]", trimmed, k),
        None => trimmed.to_string(),
    }
}

/// Flags rclone spécifiques à Google Drive, à passer à `bisync`.
///
/// Retourne `["--drive-skip-shortcuts", "--drive-skip-gdocs"]` uniquement pour
/// les remotes Drive, `[]` sinon (S3, Dropbox, local...). Passer ces flags à
/// un remote non-Drive est au mieux inutile, au pire rejeté par rclone.
pub fn drive_extra_args(remote: &str) -> Vec<String> {
    if is_drive_remote(remote) {
        vec![
            "--drive-skip-shortcuts".to_string(),
            "--drive-skip-gdocs".to_string(),
        ]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONF: &str = r#"
[GoogleDrive]
type = drive
client_id = xxx.apps.googleusercontent.com
scope = drive

[MonS3]
type = s3
provider = AWS

[LocalBackup]
type = local
"#;

    #[test]
    fn test_parse_remotes() {
        let remotes = parse_remotes(SAMPLE_CONF);
        assert_eq!(remotes.len(), 3);
        assert_eq!(remotes[0].name, "GoogleDrive");
        assert_eq!(remotes[0].kind.as_deref(), Some("drive"));
        assert!(remotes[0].is_drive());
        assert_eq!(remotes[1].kind.as_deref(), Some("s3"));
        assert!(!remotes[1].is_drive());
    }

    #[test]
    fn test_drive_extra_args_logic() {
        // Logique pure : seuls les remotes drive reçoivent les flags.
        let drive = RemoteInfo { name: "gdrive".into(), kind: Some("drive".into()) };
        let s3 = RemoteInfo { name: "s3".into(), kind: Some("s3".into()) };
        assert!(drive.is_drive());
        assert!(!s3.is_drive());
        assert_eq!(drive.display_name(), "gdrive:");
        assert_eq!(drive.human_label(), "gdrive: (Google Drive)");
        assert_eq!(s3.human_label(), "s3: (s3)");
    }
}
