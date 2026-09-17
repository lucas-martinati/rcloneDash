use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Default)]
pub struct TransferStats {
    pub bytes_done: String,
    pub bytes_total: String,
    pub pct: u8,
    pub speed: String,
    pub eta: String,
    pub files_done: u32,
    pub files_total: u32,
    pub checks_done: u32,
    pub checks_total: u32,
    pub elapsed: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ActiveFile {
    pub name: String,
    pub pct: u8,
    pub size: String,
    pub speed: String,
    pub eta: String,
    pub status: String,
    pub last_seen: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct SyncedFile {
    pub path: String,
    pub action: String, // "new", "modified", "deleted"
    pub time: String,
}

// Lazy static regexes
static RE_SYNCED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"INFO\s+:\s+(.*?):\s+(Copied \(new\)|Copied \(replaced existing\)|Updated modification time in destination|Deleted|Updated file)").unwrap()
});

static RE_TRANSFER_BYTES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"transferred:\s+([\d.]+\s*\S+)\s*/\s*([\d.]+\s*\S+),\s*(\d+)%").unwrap()
});

static RE_SPEED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\d.]+\s*\S+/s)").unwrap()
});

static RE_ETA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ETA\s+(\S+)").unwrap()
});

static RE_FILES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"transferred:\s+(\d+)\s*/\s*(\d+),").unwrap()
});

static RE_CHECKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"checks:\s+(\d+)\s*/\s*(\d+)").unwrap()
});

static RE_ELAPSED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"elapsed time:\s*(\S+)").unwrap()
});

static RE_ACTIVE_FULL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\*\s+(.+?):\s*(\d+)%\s*/([^,]+),\s*([^,]+),\s*(\S+)").unwrap()
});

static RE_ACTIVE_SHORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\*\s+(.+?):\s*(\d+)%\s*/(\S+)").unwrap()
});

static RE_ACTIVE_STATUS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\*\s+(.+?):\s*(checking|transferring)\s*$").unwrap()
});

pub fn parse_synced_file(line: &str) -> Option<SyncedFile> {
    if let Some(caps) = RE_SYNCED.captures(line) {
        let fpath = caps.get(1)?.as_str().trim().to_string();
        let act = caps.get(2)?.as_str().trim();
        let action = if act.contains("Copied (new)") {
            "new".to_string()
        } else if act.contains("Deleted") {
            "deleted".to_string()
        } else {
            "modified".to_string()
        };
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        return Some(SyncedFile {
            path: fpath,
            action,
            time: now,
        });
    }
    None
}

pub fn parse_transfer_stats(line: &str, stats: &mut TransferStats) {
    let ll = line.to_lowercase();

    if let Some(caps) = RE_TRANSFER_BYTES.captures(&ll) {
        if let (Some(d), Some(t), Some(p)) = (caps.get(1), caps.get(2), caps.get(3)) {
            stats.bytes_done = d.as_str().to_string();
            stats.bytes_total = t.as_str().to_string();
            stats.pct = p.as_str().parse().unwrap_or(0);
        }
        if let Some(sm) = RE_SPEED.captures(line) {
            if let Some(s) = sm.get(1) {
                stats.speed = s.as_str().to_string();
            }
        }
        if let Some(em) = RE_ETA.captures(line) {
            if let Some(e) = em.get(1) {
                stats.eta = e.as_str().to_string();
            }
        }
    }

    if let Some(caps) = RE_FILES.captures(&ll) {
        if let (Some(d), Some(t)) = (caps.get(1), caps.get(2)) {
            stats.files_done = d.as_str().parse().unwrap_or(0);
            stats.files_total = t.as_str().parse().unwrap_or(0);
        }
    }

    if let Some(caps) = RE_CHECKS.captures(&ll) {
        if let (Some(d), Some(t)) = (caps.get(1), caps.get(2)) {
            stats.checks_done = d.as_str().parse().unwrap_or(0);
            stats.checks_total = t.as_str().parse().unwrap_or(0);
        }
    }

    if let Some(caps) = RE_ELAPSED.captures(&ll) {
        if let Some(e) = caps.get(1) {
            stats.elapsed = e.as_str().to_string();
        }
    }
}

pub fn parse_active_file(line: &str) -> Option<ActiveFile> {
    if let Some(caps) = RE_ACTIVE_FULL.captures(line) {
        let name = caps.get(1)?.as_str().trim().to_string();
        let pct = caps.get(2)?.as_str().parse().unwrap_or(0);
        let size = caps.get(3)?.as_str().trim().to_string();
        let speed = caps.get(4)?.as_str().trim().to_string();
        let eta = caps.get(5)?.as_str().trim().to_string();
        return Some(ActiveFile {
            name,
            pct,
            size,
            speed,
            eta,
            status: "transferring".to_string(),
            last_seen: std::time::Instant::now(),
        });
    }

    if let Some(caps) = RE_ACTIVE_SHORT.captures(line) {
        let name = caps.get(1)?.as_str().trim().to_string();
        let pct = caps.get(2)?.as_str().parse().unwrap_or(0);
        let size = caps.get(3)?.as_str().trim().to_string();
        return Some(ActiveFile {
            name,
            pct,
            size,
            speed: "".to_string(),
            eta: "".to_string(),
            status: "transferring".to_string(),
            last_seen: std::time::Instant::now(),
        });
    }

    if let Some(caps) = RE_ACTIVE_STATUS.captures(line) {
        let name = caps.get(1)?.as_str().trim().to_string();
        let status = caps.get(2)?.as_str().trim().to_string();
        return Some(ActiveFile {
            name,
            pct: 0,
            size: "".to_string(),
            speed: "".to_string(),
            eta: "".to_string(),
            status,
            last_seen: std::time::Instant::now(),
        });
    }

    None
}

pub fn is_resync_trigger(line: &str) -> bool {
    let ll = line.to_lowercase();
    ll.contains("cannot find prior")
        || ll.contains("must run --resync")
        || ll.contains("--resync to recover")
        || ll.contains("path1 and path2 are out of sync")
        || ll.contains("prior or current is not in sync")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_synced_files() {
        let l1 = "2026/09/17 21:30:00 INFO  : Documents/rapport.pdf: Copied (new)";
        let res1 = parse_synced_file(l1).expect("should parse new file");
        assert_eq!(res1.path, "Documents/rapport.pdf");
        assert_eq!(res1.action, "new");

        let l2 = "2026/09/17 21:30:01 INFO  : Images/photo.jpg: Copied (replaced existing)";
        let res2 = parse_synced_file(l2).expect("should parse modified file");
        assert_eq!(res2.path, "Images/photo.jpg");
        assert_eq!(res2.action, "modified");

        let l3 = "2026/09/17 21:30:02 INFO  : Old/archive.zip: Deleted";
        let res3 = parse_synced_file(l3).expect("should parse deleted file");
        assert_eq!(res3.path, "Old/archive.zip");
        assert_eq!(res3.action, "deleted");

        let l4 = "random irrelevant log line";
        assert!(parse_synced_file(l4).is_none());
    }

    #[test]
    fn test_parse_transfer_stats() {
        let mut stats = TransferStats::default();

        parse_transfer_stats(
            "Transferred:   45.200 MiB / 120.500 MiB, 37%, 2.400 MiB/s, ETA 31s",
            &mut stats,
        );
        assert_eq!(stats.bytes_done, "45.200 mib");
        assert_eq!(stats.bytes_total, "120.500 mib");
        assert_eq!(stats.pct, 37);
        assert_eq!(stats.speed, "2.400 MiB/s");
        assert_eq!(stats.eta, "31s");

        parse_transfer_stats("Transferred:             4 / 12, 33%", &mut stats);
        assert_eq!(stats.files_done, 4);
        assert_eq!(stats.files_total, 12);

        parse_transfer_stats("Checks:                128 / 128, 100%", &mut stats);
        assert_eq!(stats.checks_done, 128);
        assert_eq!(stats.checks_total, 128);

        parse_transfer_stats("Elapsed time:        1m12.3s", &mut stats);
        assert_eq!(stats.elapsed, "1m12.3s");
    }

    #[test]
    fn test_parse_active_file() {
        let l1 = "* Documents/video.mp4: 45% /1.234Mi, 4.5Mi/s, 2m3s";
        let res1 = parse_active_file(l1).expect("should parse full active transfer");
        assert_eq!(res1.name, "Documents/video.mp4");
        assert_eq!(res1.pct, 45);
        assert_eq!(res1.size, "1.234Mi");
        assert_eq!(res1.speed, "4.5Mi/s");
        assert_eq!(res1.eta, "2m3s");

        let l2 = "* Audio/album.flac: checking";
        let res2 = parse_active_file(l2).expect("should parse checking file");
        assert_eq!(res2.name, "Audio/album.flac");
        assert_eq!(res2.status, "checking");
        assert_eq!(res2.pct, 0);
    }

    #[test]
    fn test_resync_trigger() {
        assert!(is_resync_trigger("ERROR : Bisync error: must run --resync to recover"));
        assert!(is_resync_trigger("Fatal: Path1 and Path2 are out of sync"));
        assert!(!is_resync_trigger("INFO  : Bisync successful"));
    }
}

