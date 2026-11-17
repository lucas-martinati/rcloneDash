use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
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

pub fn format_bytes(b: u64) -> String {
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

impl FileEntry {
    pub fn size_formatted(&self) -> String {
        if self.is_dir {
            return "<REP>".to_string();
        }
        format_bytes(self.size)
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
            name: ".. (Parent folder)".to_string(),
            rel_path: "..".to_string(),
            is_dir: true,
            size: 0,
            mtime: "".to_string(),
            ignored: false,
        });
    }

    let compiled_filters = CompiledFilters::new(filters);
    let rd = fs::read_dir(&full_target)?;
    for entry_res in rd {
        let entry = entry_res?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Ignorer seulement . et ..
        if file_name == "." || file_name == ".." {
            continue;
        }

        let full_path = entry.path();
        let metadata = fs::metadata(&full_path).ok();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        let mtime = metadata
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_default();

        let item_rel = if rel.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", rel, file_name)
        };

        let ignored = compiled_filters.is_ignored(&item_rel, is_dir);

        entries.push(FileEntry {
            name: file_name,
            rel_path: item_rel,
            is_dir,
            size,
            mtime,
            ignored,
        });
    }

    // Tri : parent ".." en tête, puis dossiers, puis fichiers, par ordre alphabétique
    entries.sort_by_cached_key(|e| {
        let rank = if e.name.starts_with("..") {
            0u8
        } else if e.is_dir {
            1u8
        } else {
            2u8
        };
        (rank, e.name.to_lowercase())
    });

    Ok(entries)
}

pub fn glob_to_regex(pat: &str) -> String {
    let mut out = String::from("^");
    let chars: Vec<char> = pat.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        if i + 2 < n && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '/' {
            out.push_str("(?:.*/)?");
            i += 3;
        } else if i + 1 < n && chars[i] == '*' && chars[i + 1] == '*' {
            out.push_str(".*");
            i += 2;
        } else if chars[i] == '*' {
            out.push_str("[^/]*");
            i += 1;
        } else if chars[i] == '?' {
            out.push_str("[^/]");
            i += 1;
        } else {
            let mut buf = [0u8; 4];
            out.push_str(&regex::escape(chars[i].encode_utf8(&mut buf)));
            i += 1;
        }
    }
    out.push('$');
    out
}

/// Zero-allocation subpath matcher: tests `clean_path` directly, and when unanchored,
/// tests every suffix starting after a `'/'` separator.
fn match_any_level(rx: &regex::Regex, clean_path: &str, anchored: bool) -> bool {
    if rx.is_match(clean_path) {
        return true;
    }
    if anchored {
        return false;
    }
    for (idx, ch) in clean_path.char_indices() {
        if ch == '/' && rx.is_match(&clean_path[idx + 1..]) {
            return true;
        }
    }
    false
}

struct CompiledRule {
    anchored: bool,
    rx_full: regex::Regex,
    rx_sub_all: Option<regex::Regex>,
    rx_dir_slash: Option<regex::Regex>,
}

/// Pre-compiled set of rclone exclusion filters. Compiling once per directory
/// listing turns `O(files * filters)` regex builds into `O(filters)`.
pub struct CompiledFilters {
    rules: Vec<CompiledRule>,
}

impl CompiledFilters {
    pub fn new(filters: &[String]) -> Self {
        let mut rules = Vec::new();
        for rule in filters {
            let trimmed = rule.trim();
            let pat = if let Some(stripped) = trimmed.strip_prefix("- ") {
                stripped.trim()
            } else if let Some(stripped) = trimmed.strip_prefix('-') {
                stripped.trim()
            } else {
                continue;
            };

            if pat.is_empty() {
                continue;
            }

            let anchored = pat.starts_with('/');
            let body = if anchored { pat.trim_start_matches('/') } else { pat };
            if body.is_empty() {
                continue;
            }

            let Ok(rx_full) = regex::Regex::new(&glob_to_regex(body)) else {
                continue;
            };
            let rx_sub_all = body
                .strip_suffix("/**")
                .and_then(|base| regex::Regex::new(&glob_to_regex(base)).ok());
            let rx_dir_slash = body
                .strip_suffix('/')
                .and_then(|base| regex::Regex::new(&glob_to_regex(base)).ok());

            rules.push(CompiledRule {
                anchored,
                rx_full,
                rx_sub_all,
                rx_dir_slash,
            });
        }
        Self { rules }
    }

    pub fn is_ignored(&self, rel_path: &str, is_dir: bool) -> bool {
        let clean = rel_path.trim_matches('/');
        if clean.is_empty() {
            return false;
        }

        for rule in &self.rules {
            if match_any_level(&rule.rx_full, clean, rule.anchored) {
                return true;
            }
            // « base/** » : le dossier « base » lui-même est aussi considéré exclu
            if let Some(ref rx_base) = rule.rx_sub_all {
                if match_any_level(rx_base, clean, rule.anchored) {
                    return true;
                }
            }
            // Si c'est un dossier et que le motif se termine par '/', tester la base
            if is_dir {
                if let Some(ref rx_dir) = rule.rx_dir_slash {
                    if match_any_level(rx_dir, clean, rule.anchored) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
pub fn is_path_ignored(rel_path: &str, is_dir: bool, filters: &[String]) -> bool {
    CompiledFilters::new(filters).is_ignored(rel_path, is_dir)
}

pub fn open_with_xdg(base: &Path, rel_path: &str) -> Result<(), String> {
    let clean = rel_path.trim_start_matches('/');
    let full = if clean.is_empty() {
        base.to_path_buf()
    } else {
        base.join(clean)
    };
    if !full.exists() {
        return Err("File deleted or not found".to_string());
    }
    crate::cmd::open_path(&full)
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

    crate::cmd::open_path(&target)
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
        return Err("File or folder does not exist".to_string());
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

        // Test parité web : règles sans slash initial (node_modules/**, target/**, venv/**)
        let rclone_filters = vec![
            "- venv/**".to_string(),
            "- node_modules/**".to_string(),
            "- target/**".to_string(),
            "- **/.DS_Store".to_string(),
            "- Cours/2GT7/**".to_string(),
        ];

        // Le dossier lui-même doit être exclu !
        assert!(is_path_ignored("venv", true, &rclone_filters));
        assert!(is_path_ignored("node_modules", true, &rclone_filters));
        assert!(is_path_ignored("target", true, &rclone_filters));
        assert!(is_path_ignored("Cours/2GT7", true, &rclone_filters));

        // Dossiers exclus imbriqués
        assert!(is_path_ignored("projets/mon_app/node_modules", true, &rclone_filters));
        assert!(is_path_ignored("projets/mon_app/node_modules/index.js", false, &rclone_filters));
        assert!(is_path_ignored("projets/sub/target", true, &rclone_filters));
        assert!(is_path_ignored("sous_dossier/venv/bin/python", false, &rclone_filters));

        // Motifs **/
        assert!(is_path_ignored(".DS_Store", false, &rclone_filters));
        assert!(is_path_ignored("Images/.DS_Store", false, &rclone_filters));

        // Fichiers autorisés
        assert!(!is_path_ignored("Cours/Autre/fichier.pdf", false, &rclone_filters));
        assert!(!is_path_ignored("projets/mon_app/src/main.rs", false, &rclone_filters));
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

    #[test]
    fn test_list_directory_includes_hidden_files() {
        let temp_dir = std::env::temp_dir().join("rclonedash_test_hidden");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join(".env"), "SECRET=123").unwrap();
        std::fs::create_dir_all(temp_dir.join(".hidden_folder")).unwrap();
        std::fs::write(temp_dir.join("visible.txt"), "hello").unwrap();

        let entries = list_directory(&temp_dir, "", &[]).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&".env"), "Les fichiers cachés doivent être inclus");
        assert!(names.contains(&".hidden_folder"), "Les dossiers cachés doivent être inclus");
        assert!(names.contains(&"visible.txt"), "Les fichiers normaux doivent être inclus");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

