/* ═══════════════════════════════════════════════════
   STATE
   ═══════════════════════════════════════════════════ */
export const S = {
  theme: document.documentElement.dataset.theme || 'dark',
  llc: 0,
  interval: null,
  curState: '',
  logFilter: 'all',
  lastLogs: '',
  nextSyncTs: 0, // timestamp (ms) de la prochaine sync planifiée
  lastStartTs: 0, // timestamp (ms) du dernier déclenchement
  isSyncing: false,
  livePct: -1, // % de transfert connu pendant une sync
  runsSig: '', // signatures des dernières données rendues,
  recentSig: '' // pour ne pas reconstruire le DOM inutilement
};
