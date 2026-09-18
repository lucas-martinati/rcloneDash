use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use crate::config;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub rel_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: String,
    pub ignored: bool,
}

impl FileEntry {
    pub fn size_formatted(&self) -> String {
        if self.is_dir {
            return "<REP>".to_string();
        }
        let b = self.size;
        if b >= 1024 * 1024 * 1024 {
            format!("{:.1} Go", b as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if b >= 1024 * 1024 {
            format!("{:.1} Mo", b as f64 / (1024.0 * 1024.0))
        } else if b >= 1024 {
            format!("{:.1} Ko", b as f64 / 1024.0)
        } else {
            format!("{} o", b)
        }
    }
}

pub fn list_directory(base: &Path, rel: &str, filters: &[String]) -> Result<Vec<FileEntry>, std::io::Error> {
    let full_target = if rel.is_empty() {
        base.to_path_buf()
    } else {
        base.join(rel)
    };

    if !full_target.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    // Entrée pour remonter dans le dossier parent si on est dans un sous-dossier
    if !rel.is_empty() {
        entries.push(FileEntry {
            name: ".. (Dossier parent)".to_string(),
            rel_path: "..".to_string(),
            is_dir: true,
            size: 0,
            mtime: "".to_string(),
            ignored: false,
        });
    }

    let rd = fs::read_dir(&full_target)?;
    for entry_res in rd {
        let entry = entry_res?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Ignorer fichiers cachés système (.git, etc.)
        if file_name.starts_with('.') && file_name != ".nomedia" {
            continue;
        }

        let full_path = entry.path();
        let is_dir = full_path.is_dir();
        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        let mtime = metadata.and_then(|m| m.modified().ok()).map(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%Y-%m-%d %H:%M").to_string()
        }).unwrap_or_default();

        let item_rel = if rel.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", rel, file_name)
        };

        let ignored = is_path_ignored(&item_rel, is_dir, filters);

        entries.push(FileEntry {
            name: file_name,
            rel_path: item_rel,
            is_dir,
            size,
            mtime,
            ignored,
        });
    }

    // Tri : dossiers d'abord, puis fichiers, par ordre alphabétique
    entries.sort_by(|a, b| {
        if a.name.starts_with("..") {
            std::cmp::Ordering::Less
        } else if b.name.starts_with("..") {
            std::cmp::Ordering::Greater
        } else if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(entries)
}

pub fn is_path_ignored(rel_path: &str, is_dir: bool, filters: &[String]) -> bool {
    let normalized = format!("/{}", rel_path.trim_start_matches('/'));
    for rule in filters {
        let trimmed = rule.trim();
        if trimmed.starts_with('-') {
            let pat = trimmed[1..].trim();
            if pat.ends_with("/**") {
                let dir_pat = &pat[..pat.len() - 3];
                if normalized.starts_with(dir_pat) {
                    return true;
                }
            } else if pat.starts_with("*.") {
                let ext = &pat[1..];
                if normalized.ends_with(ext) {
                    return true;
                }
            } else if pat == normalized || (is_dir && normalized.starts_with(pat)) {
                return true;
            }
        }
    }
    false
}

pub fn open_with_xdg(base: &Path, rel_path: &str) -> Result<(), String> {
    let clean = rel_path.trim_start_matches('/');
    let full = if clean.is_empty() {
        base.to_path_buf()
    } else {
        base.join(clean)
    };
    Command::new("xdg-open")
        .arg(&full)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn open_folder_with_xdg(base: &Path, rel_path: &str) -> Result<(), String> {
    let clean = rel_path.trim_start_matches('/');
    let full = if clean.is_empty() {
        base.to_path_buf()
    } else {
        base.join(clean)
    };

    // Trouver un dossier valide : soit le dossier lui-même, soit le parent existant le plus proche
    let mut target = if full.is_dir() && full.exists() {
        full
    } else if let Some(p) = full.parent() {
        p.to_path_buf()
    } else {
        base.to_path_buf()
    };

    while !target.exists() {
        if let Some(p) = target.parent() {
            if p.starts_with(base) {
                target = p.to_path_buf();
            } else {
                target = base.to_path_buf();
                break;
            }
        } else {
            target = base.to_path_buf();
            break;
        }
    }

    Command::new("xdg-open")
        .arg(&target)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn add_exclude_rule(rel_path: &str, is_dir: bool) -> Result<(), String> {
    let rule = if is_dir {
        format!("- /{rel_path}/**\n")
    } else {
        format!("- /{rel_path}\n")
    };

    let filters_file = config::filters_file();
    if let Some(parent) = filters_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filters_file)
        .map_err(|e| e.to_string())?;

    file.write_all(rule.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_entry(base: &Path, rel_path: &str) -> Result<(), String> {
    let full = base.join(rel_path);
    if !full.exists() {
        return Err("Le fichier ou dossier n'existe pas".to_string());
    }

    if full.is_dir() {
        fs::remove_dir_all(&full).map_err(|e| e.to_string())
    } else {
        fs::remove_file(&full).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_path_ignored() {
        let filters = vec![
            "- /Cache/**".to_string(),
            "- *.tmp".to_string(),
            "- /secret.txt".to_string(),
            "+ /important/**".to_string(),
        ];

        assert!(is_path_ignored("Cache/file.dat", false, &filters));
        assert!(is_path_ignored("Cache/sub/file.dat", false, &filters));
        assert!(is_path_ignored("test.tmp", false, &filters));
        assert!(is_path_ignored("secret.txt", false, &filters));

        assert!(!is_path_ignored("Documents/rapport.pdf", false, &filters));
        assert!(!is_path_ignored("important/notes.txt", false, &filters));
    }

    #[test]
    fn test_size_formatting() {
        let e_dir = FileEntry {
            name: "Dossier".to_string(),
            rel_path: "Dossier".to_string(),
            is_dir: true,
            size: 0,
            mtime: "".to_string(),
            ignored: false,
        };
        assert_eq!(e_dir.size_formatted(), "<REP>");

        let e_file = FileEntry {
            name: "video.mp4".to_string(),
            rel_path: "video.mp4".to_string(),
            is_dir: false,
            size: 15 * 1024 * 1024,
            mtime: "".to_string(),
            ignored: false,
        };
        assert_eq!(e_file.size_formatted(), "15.0 Mo");
    }
}

