use std::process::Command;
use crate::monitor::parser::{is_resync_trigger, parse_log_line, FileAction, SyncEvent, SyncedFile};

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

        // Start of a new execution
        let is_systemd_start = ll.contains("systemd")
            && (ll.contains("starting") || ll.contains("started"))
            && ll.contains("rclone-bisync");

        let is_guard_start = !in_run
            && (ll.contains("rclone-bisync-guard") || ll.contains("rclonedash"))
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

            // End of execution
            let is_finish = (ll.contains("systemd") && (ll.contains("finished") || ll.contains("deactivated")) && ll.contains("rclone-bisync"))
                || ll.contains("bisync successful")
                || ((ll.contains("rclone-bisync-guard") || ll.contains("rclonedash")) && (ll.contains("aucun changement") || ll.contains("sync ignoré")));

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

    // Most recent first
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

    // Extract date/time (ISO timestamp at start of line: 2026-09-17T21:45:00+0200)
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

        if ll.contains("aucun changement local") || ll.contains("sync ignoré") || ll.contains("garde légère") {
            status = RunStatus::Skipped;
        } else if (ll.contains("error :") || ll.contains("fatal error") || ll.contains("failed") || is_resync_trigger(line)) && !ll.contains("0 errors") {
            status = RunStatus::Failed;
            errors.push(line.to_string());
        }

        let events = parse_log_line(line);
        for event in events {
            match event {
                SyncEvent::FileSynced(SyncedFile { path, action, .. }) => {
                    match action {
                        FileAction::New | FileAction::Copied => {
                            if !copied.contains(&path) {
                                copied.push(path.clone());
                            }
                        }
                        FileAction::Deleted => {
                            if !deleted.contains(&path) {
                                deleted.push(path.clone());
                            }
                        }
                        FileAction::Modified => {
                            if !modified.contains(&path) {
                                modified.push(path.clone());
                            }
                        }
                    }
                    if !synced_files.iter().any(|(_, p, _)| p == &path) {
                        synced_files.push((action.to_string(), path, time.clone()));
                    }
                }
                SyncEvent::StatsUpdated(st) => {
                    if !st.elapsed.is_empty() && st.elapsed != "0.0s" {
                        duration = st.elapsed;
                    }
                }
                SyncEvent::ResyncRequired(err) => {
                    status = RunStatus::Failed;
                    if !errors.contains(&err) {
                        errors.push(err);
                    }
                }
                SyncEvent::SyncCompleted { success: false, error_msg } => {
                    status = RunStatus::Failed;
                    if let Some(e) = error_msg {
                        if !errors.contains(&e) {
                            errors.push(e);
                        }
                    }
                }
                _ => {}
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
    fn test_parse_journal_history_legacy_text() {
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
        assert_eq!(runs.len(), 1, "Ignored syncs / empty wakeups must be filtered from history");

        assert_eq!(runs[0].status, RunStatus::Success);
        assert_eq!(runs[0].files_copied.len(), 1);
        assert_eq!(runs[0].files_copied[0], "Documents/notes.txt");
        assert_eq!(runs[0].duration, "4.2s");
    }

    #[test]
    fn test_parse_journal_history_json_logs() {
        let sample_json_journal = r#"
2026-09-21T09:40:00+0200 mypc systemd[1]: Starting rclone-bisync.service...
2026-09-21T09:40:01+0200 mypc rclone-bisync-guard[1234]: RcloneDash: lancement du bisync — changement local
2026-09-21T09:40:02+0200 mypc rclone[1235]: {"time":"2026-09-21T09:40:02+02:00","level":"info","msg":"Synching Path1 with Path2"}
2026-09-21T09:40:03+0200 mypc rclone[1235]: {"time":"2026-09-21T09:40:03+02:00","level":"info","msg":"Copied (new)","size":100,"object":"reports/q3.xlsx"}
2026-09-21T09:40:04+0200 mypc rclone[1235]: {"time":"2026-09-21T09:40:04+02:00","level":"info","msg":"\nTransferred: 100 / 100\nChecks: 2 / 2\nElapsed time:         1.8s\n\n","stats":{"bytes":100,"checks":2,"elapsedTime":1.8,"errors":0,"speed":500.0,"totalBytes":100,"totalChecks":2,"totalTransfers":1,"transfers":1}}
2026-09-21T09:40:05+0200 mypc rclone[1235]: {"time":"2026-09-21T09:40:05+02:00","level":"info","msg":"Bisync successful"}
2026-09-21T09:40:06+0200 mypc systemd[1]: Finished rclone-bisync.service.
"#;

        let runs = parse_journal_history(sample_json_journal, 10);
        assert_eq!(runs.len(), 1, "Should parse JSON run from journalctl");
        assert_eq!(runs[0].status, RunStatus::Success);
        assert_eq!(runs[0].files_copied.len(), 1);
        assert_eq!(runs[0].files_copied[0], "reports/q3.xlsx");
        assert_eq!(runs[0].duration, "1.8s");
    }

    #[test]
    fn test_in_progress_sync_not_added_to_history() {
        let sample_journal = r#"
2026-09-17T20:10:00+0200 mypc systemd[1]: Starting rclone-bisync.service...
2026-09-17T20:10:05+0200 mypc rclone[1235]: Bisync successful
2026-09-17T20:10:06+0200 mypc systemd[1]: Finished rclone-bisync.service.
2026-09-17T20:30:00+0200 mypc systemd[1]: Starting rclone-bisync.service...
2026-09-17T20:30:01+0200 mypc rclone-bisync-guard[1400]: Lancement du bisync
2026-09-17T20:30:02+0200 mypc rclone[1401]: Synching Path1 with Path2
2026-09-17T20:30:03+0200 mypc rclone[1401]: INFO  : Photos/vacances.jpg: Copied (new)
"#;

        let runs = parse_journal_history(sample_journal, 10);
        assert_eq!(runs.len(), 1, "The ongoing in-progress sync must not appear in past history runs");
        assert_eq!(runs[0].time, "20:10:00");
    }
}
