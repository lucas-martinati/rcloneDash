use std::process::Command;
use crate::monitor::parser::{parse_synced_file, is_resync_trigger};

#[derive(Debug, Clone)]
pub struct PastRun {
    pub id: usize,
    pub date: String,
    pub time: String,
    pub duration: String,
    pub status: RunStatus,
    pub files_copied: Vec<String>,
    pub files_modified: Vec<String>,
    pub files_deleted: Vec<String>,
    pub synced_files: Vec<(String, String, String)>,
    pub errors: Vec<String>,
}

impl PastRun {
    pub fn all_affected_files(&self) -> Vec<(&str, &str)> {
        let mut list = Vec::new();
        for (act, path, _) in &self.synced_files {
            list.push((act.as_str(), path.as_str()));
        }
        for f in &self.files_copied {
            if !list.iter().any(|(_, p)| p == &f.as_str()) {
                list.push(("new", f.as_str()));
            }
        }
        for f in &self.files_modified {
            if !list.iter().any(|(_, p)| p == &f.as_str()) {
                list.push(("modified", f.as_str()));
            }
        }
        for f in &self.files_deleted {
            if !list.iter().any(|(_, p)| p == &f.as_str()) {
                list.push(("deleted", f.as_str()));
            }
        }
        list
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Success,
    Failed,
    Skipped,
}

pub fn fetch_past_runs(limit: usize) -> Vec<PastRun> {
    let output = Command::new("journalctl")
        .args([
            "--user",
            "-u",
            "rclone-bisync.service",
            "--output=short-iso",
            "-n",
            "2500",
        ])
        .output();

    let text = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    parse_journal_history(&text, limit)
}

pub fn parse_journal_history(journal_text: &str, limit: usize) -> Vec<PastRun> {
    let mut runs = Vec::new();
    let lines: Vec<&str> = journal_text.lines().collect();

    let mut current_lines: Vec<&str> = Vec::new();
    let mut in_run = false;

    for line in lines {
        let ll = line.to_lowercase();

        // Début d'une nouvelle exécution
        let is_systemd_start = ll.contains("systemd")
            && (ll.contains("starting") || ll.contains("started"))
            && ll.contains("rclone-bisync");

        let is_guard_start = !in_run
            && ll.contains("rclone-bisync-guard")
            && (ll.contains("lancement du bisync") || ll.contains("aucun changement") || ll.contains("sync ignoré"));

        if is_systemd_start || is_guard_start {
            if in_run && !current_lines.is_empty() {
                if let Some(run) = analyze_run(&current_lines, runs.len() + 1) {
                    if run.status != RunStatus::Skipped {
                        runs.push(run);
                    }
                }
                current_lines.clear();
            }
            in_run = true;
        }

        if in_run {
            current_lines.push(line);

            // Fin d'exécution
            let is_finish = (ll.contains("systemd") && (ll.contains("finished") || ll.contains("deactivated")) && ll.contains("rclone-bisync"))
                || ll.contains("bisync successful")
                || (ll.contains("rclone-bisync-guard") && (ll.contains("aucun changement") || ll.contains("sync ignoré")));

            if is_finish {
                if let Some(run) = analyze_run(&current_lines, runs.len() + 1) {
                    if run.status != RunStatus::Skipped {
                        runs.push(run);
                    }
                }
                current_lines.clear();
                in_run = false;
            }
        }
    }

    if in_run && !current_lines.is_empty() {
        if let Some(run) = analyze_run(&current_lines, runs.len() + 1) {
            if run.status != RunStatus::Skipped {
                runs.push(run);
            }
        }
    }

    // Plus récents en premier
    runs.reverse();
    if runs.len() > limit {
        runs.truncate(limit);
    }

    runs
}

fn analyze_run(lines: &[&str], id: usize) -> Option<PastRun> {
    if lines.is_empty() {
        return None;
    }

    let first_line = lines[0];
    let mut date = "--".to_string();
    let mut time = "--".to_string();

    // Extraction date/heure (ISO timestamp en début de ligne: 2026-09-17T21:45:00+0200)
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if let Some(ts) = parts.first() {
        if let Some((d, t)) = ts.split_once('T') {
            date = d.to_string();
            time = t.chars().take(8).collect();
        }
    }

    let mut status = RunStatus::Success;
    let mut copied = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();
    let mut synced_files = Vec::new();
    let mut errors = Vec::new();
    let mut duration = "< 1s".to_string();

    for line in lines {
        let ll = line.to_lowercase();

        if ll.contains("aucun changement local") || ll.contains("ignoré") || ll.contains("garde légère") {
            status = RunStatus::Skipped;
        } else if ll.contains("error :") || ll.contains("fatal error") || ll.contains("failed") || is_resync_trigger(line) {
            if !ll.contains("0 errors") {
                status = RunStatus::Failed;
                errors.push(line.to_string());
            }
        }

        if let Some(synced) = parse_synced_file(line) {
            match synced.action.as_str() {
                "new" => copied.push(synced.path.clone()),
                "deleted" => deleted.push(synced.path.clone()),
                _ => modified.push(synced.path.clone()),
            }
            synced_files.push((synced.action, synced.path, time.clone()));
        }

        if ll.contains("elapsed time:") {
            if let Some(idx) = line.find("Elapsed time:") {
                let rest = &line[idx + 13..].trim();
                let dur: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                if !dur.is_empty() {
                    duration = dur;
                }
            }
        }
    }

    Some(PastRun {
        id,
        date,
        time,
        duration,
        status,
        files_copied: copied,
        files_modified: modified,
        files_deleted: deleted,
        synced_files,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_journal_history() {
        let sample_journal = r#"
2026-09-17T20:10:00+0200 mypc systemd[1]: Starting rclone-bisync.service...
2026-09-17T20:10:01+0200 mypc rclone-bisync-guard[1234]: Lancement du bisync
2026-09-17T20:10:02+0200 mypc rclone[1235]: Synching Path1 with Path2
2026-09-17T20:10:03+0200 mypc rclone[1235]: INFO  : Documents/notes.txt: Copied (new)
2026-09-17T20:10:04+0200 mypc rclone[1235]: Elapsed time: 4.2s
2026-09-17T20:10:05+0200 mypc rclone[1235]: Bisync successful
2026-09-17T20:20:00+0200 mypc rclone-bisync-guard[1300]: Aucun changement local détecté, sync ignoré (garde légère)
"#;

        let runs = parse_journal_history(sample_journal, 10);
        assert_eq!(runs.len(), 1, "Les syncs ignorées / réveils à vide doivent être filtrés de l'historique");

        // Seul le vrai run (Success avec 1 fichier nouveau copié) est conservé
        assert_eq!(runs[0].status, RunStatus::Success);
        assert_eq!(runs[0].files_copied.len(), 1);
        assert_eq!(runs[0].files_copied[0], "Documents/notes.txt");
        assert_eq!(runs[0].duration, "4.2s");
    }
}

