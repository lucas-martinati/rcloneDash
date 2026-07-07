/* ═══════════════════════════════════════════════════
   STATE
   ═══════════════════════════════════════════════════ */
let _theme = document.documentElement.dataset.theme || 'dark';
let _llc = '';
let _interval = null;
let _curState = '';
let _logFilter = 'all';
let _lastLogs = [];
let _nextSyncTs = null;   // timestamp (ms) de la prochaine sync planifiée
let _lastStartTs = null;  // timestamp (ms) du dernier déclenchement
let _isSyncing = false;
let _livePct = null;      // % de transfert connu pendant une sync
let _runsSig = '';        // signatures des dernières données rendues,
let _recentSig = '';      // pour ne pas reconstruire le DOM inutilement
