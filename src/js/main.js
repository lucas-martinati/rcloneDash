/* ═══════════════════════════════════════════════════
   ES MODULE — POINT D'ENTRÉE
   ═══════════════════════════════════════════════════ */
import { S } from './state.js';
import { applyThemeIcon, toggleTheme } from './theme.js';
import { initDragAndDrop } from './drag-resize.js';
import { refresh, doSync, cancelSync } from './refresh.js';
import { initLiveStream } from './live-stream.js';
import { tickPulse } from './pulse.js';
import { initFocusTrap } from './focus-trap.js';
import { setLogFilter, logsBot } from './logs.js';
import { filterRecent } from './recent.js';
import { showTooltip, hideTooltip } from './sparkline.js';
import { toggleRunDetails, copyErrorLogs } from './history.js';
import {
  openTreeModal,
  closeTreeModal,
  treeUp,
  loadTree,
  openCurrentDir,
  fmFilter,
  fmClearSearch,
  fmToggleRecursive,
  fmSort,
  openFile,
  _fm
} from './file-browser.js';
import { openDeleteModal, closeDeleteModal, confirmDelete, runDriveCheck } from './delete.js';
import {
  ignorePath,
  openExcludeModal,
  closeExcludeModal,
  confirmExclude,
  excOnChoice,
  reincludePath,
  closeReincModal
} from './exclude.js';
import {
  openFiltersModal,
  closeFiltersModal,
  addFilter,
  saveFilters,
  checkFiltersModified,
  openImpactModal,
  closeImpactModal,
  impOnChoice,
  confirmAddFilter
} from './filters.js';
import {
  openBwModal,
  closeBwModal,
  saveBw,
  openDryRunModal,
  closeDryRunModal,
  startDryRun
} from './modals.js';

/* ── Exposer les fonctions nécessaires aux onclick inline du HTML ── */
Object.assign(window, {
  // refresh + sync
  doSync,
  cancelSync,
  refresh,
  // theme
  toggleTheme,
  // logs
  setLogFilter,
  logsBot,
  // recent
  filterRecent,
  // sparkline tooltips
  showTooltip,
  hideTooltip,
  // history
  toggleRunDetails,
  copyErrorLogs,
  // file browser
  openTreeModal,
  closeTreeModal,
  treeUp,
  loadTree,
  openCurrentDir,
  fmFilter,
  fmClearSearch,
  fmToggleRecursive,
  fmSort,
  openFile,
  _fm,
  // delete modal
  openDeleteModal,
  closeDeleteModal,
  confirmDelete,
  runDriveCheck,
  // exclude modal
  ignorePath,
  openExcludeModal,
  closeExcludeModal,
  confirmExclude,
  excOnChoice,
  reincludePath,
  closeReincModal,
  // filters modal
  openFiltersModal,
  closeFiltersModal,
  addFilter,
  saveFilters,
  checkFiltersModified,
  openImpactModal,
  closeImpactModal,
  impOnChoice,
  confirmAddFilter,
  // bandwidth & dry run modals
  openBwModal,
  closeBwModal,
  saveBw,
  openDryRunModal,
  closeDryRunModal,
  startDryRun
});

/* ═══════════════════════════════════════════════════
   CLAVIER
   ═══════════════════════════════════════════════════ */
document.addEventListener('keydown', function (e) {
  if (e.key === 'Escape') {
    document.querySelectorAll('.modal-overlay.show').forEach(function (m) {
      m.classList.remove('show');
    });
  }
});

/* ═══════════════════════════════════════════════════
   OUVERTURE DE FICHIER — écouteur délégué
   Les lignes produites par renderFileRow portent des
   data-attributs ; un seul listener évite le onclick
   inline et son échappement de chaînes fragile.
   ═══════════════════════════════════════════════════ */
document.addEventListener('click', function (e) {
  let el = e.target.closest('.file-link[data-openfile]');
  if (el) openFile(el.dataset.fpath, el.dataset.deleted === '1', e);
});

/* ═══════════════════════════════════════════════════
   CTRL MAINTENU → « ouvrir le dossier »
   Bascule une classe sur <body> pour que le survol des
   liens de fichiers indique qu'on ouvrira le dossier parent.
   ═══════════════════════════════════════════════════ */
// Ctrl seul → seule la ligne survolée passe en mode dossier.
// Ctrl+Maj → toutes les lignes passent en mode dossier d'un coup.
function updateCtrlState(e) {
  let ctrl = e.ctrlKey || e.metaKey;
  document.body.classList.toggle('folder-mode', ctrl);
  document.body.classList.toggle('folder-mode-all', ctrl && e.shiftKey);
}
window.addEventListener('keydown', updateCtrlState);
window.addEventListener('keyup', updateCtrlState);
// Sécurité : on retire les classes si la fenêtre perd le focus (le keyup peut être manqué)
window.addEventListener('blur', function () {
  document.body.classList.remove('folder-mode', 'folder-mode-all');
});

/* ═══════════════════════════════════════════════════
   BOOT
   ═══════════════════════════════════════════════════ */
applyThemeIcon();
initDragAndDrop();
initFocusTrap();
refresh();
initLiveStream();
S.interval = setInterval(refresh, 10000);
setInterval(tickPulse, 1000);
