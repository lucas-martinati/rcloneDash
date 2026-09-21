use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    New,
    Modified,
    Deleted,
    Copied,
}

impl FileAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileAction::New => "new",
            FileAction::Modified => "modified",
            FileAction::Deleted => "deleted",
            FileAction::Copied => "copied",
        }
    }
}

impl std::fmt::Display for FileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedFileDetail {
    pub path: String,
    pub action: FileAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPhase {
    Listings,
    RemoteDiffs,
    LocalDiffs,
    Applying,
    Updating,
    Done,
}

impl SyncPhase {
    pub fn index(&self) -> usize {
        match self {
            SyncPhase::Listings => 0,
            SyncPhase::RemoteDiffs => 1,
            SyncPhase::LocalDiffs => 2,
            SyncPhase::Applying => 3,
            SyncPhase::Updating => 4,
            SyncPhase::Done => 5,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SyncPhase::Listings => "1. Listings",
            SyncPhase::RemoteDiffs => "2. Remote Diffs",
            SyncPhase::LocalDiffs => "3. Local Diffs",
            SyncPhase::Applying => "4. Applying",
            SyncPhase::Updating => "5. Updating",
            SyncPhase::Done => "6. Done",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
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

#[derive(Debug, Clone)]
pub struct ActiveFile {
    pub name: String,
    pub pct: u8,
    pub speed: String,
    pub last_seen: Instant,
}

impl PartialEq for ActiveFile {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.pct == other.pct && self.speed == other.speed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedFile {
    pub path: String,
    pub action: FileAction,
    pub time: String,
}

/// Événements universels de synchronisation produits par les adaptateurs de log
#[derive(Debug, Clone, PartialEq)]
pub enum SyncEvent {
    /// Démarrage d'un nouveau cycle de synchronisation
    SyncStarted {
        reason: Option<String>,
    },
    /// Changement de phase dans le pipeline de synchronisation
    PhaseChanged(SyncPhase),
    /// Détection de modification d'un côté (true = local / Path2, false = distant / Path1)
    PathModified {
        is_local: bool,
    },
    /// Mise à jour des métriques globales de transfert
    StatsUpdated(TransferStats),
    /// Mise à jour de la progression d'un fichier en cours de transfert
    ActiveTransferUpdated(ActiveFile),
    /// Fichier dont le transfert / la modification est terminé(e)
    FileSynced(SyncedFile),
    /// Détection d'un fichier modifié lors de la phase de diffs
    DiffFound {
        is_local: bool,
        detail: ModifiedFileDetail,
    },
    /// Erreur critique nécessitant un --resync
    ResyncRequired(String),
    /// Fin du cycle de synchronisation
    SyncCompleted {
        success: bool,
        error_msg: Option<String>,
    },
}
