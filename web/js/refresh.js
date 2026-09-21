import { spin, fmtT } from './utils.js';
import { bus } from './state.js';
import { updatePulse } from './pulse.js';
import { updateAlerts, updateKPIs } from './dashboard.js';
import { updateRuns } from './history.js';
import { updateLogs } from './logs.js';
import { updateRecentFiles } from './recent.js';
import { toast } from './toasts.js';

/* ═══════════════════════════════════════════════════
   REFRESH PRINCIPAL
   ═══════════════════════════════════════════════════ */
export async function refresh() {
  spin(true);
  try {
    let r = await fetch('/api/status');
    if (!r.ok) throw new Error(r.status);
    let d = await r.json();

    bus.emit('sync:status', d.service.state);
    updatePulse(d);
    updateAlerts(d);
    updateKPIs(d);
    if (d.live !== undefined) {
      bus.emit('live:update', d.live);
    }
    updateRuns(d.runs);
    updateLogs(d.logs);
    updateRecentFiles(d.recent_files);
    document.getElementById('ts').textContent = 'MàJ ' + fmtT(d.ts);
  } catch (e) {
    document.getElementById('ts').textContent = '⚠ serveur injoignable';
  } finally {
    spin(false);
  }
}

/* ═══════════════════════════════════════════════════
   ACTIONS SYNC — via modales de confirmation
   (composants existants : .modal-overlay/.show, .btn, toasts)
   ═══════════════════════════════════════════════════ */
export function openSyncModal() {
  document.getElementById('sync-modal')?.classList.add('show');
}
export function closeSyncModal() {
  document.getElementById('sync-modal')?.classList.remove('show');
}
export async function confirmSync() {
  closeSyncModal();
  await doSync();
}

export function openResyncModal() {
  document.getElementById('resync-modal')?.classList.add('show');
}
export function closeResyncModal() {
  document.getElementById('resync-modal')?.classList.remove('show');
}
export async function confirmResync() {
  closeResyncModal();
  await doResync();
}

export async function doSync() {
  let b = document.getElementById('bsync');
  let lbl = document.getElementById('bsync-lbl');
  let c = document.getElementById('sync-confirm-btn');
  if (b) b.disabled = true;
  if (c) c.disabled = true;
  if (lbl) lbl.textContent = 'Démarrage…';
  try {
    let r = await fetch('/api/trigger', { method: 'POST' });
    let d = await r.json();
    if (d.ok) {
      toast('Synchronisation lancée', 'ok');
    } else {
      toast('Impossible de lancer la synchronisation : ' + (d.error || 'erreur inconnue'), 'err');
    }
  } catch {
    toast('Serveur injoignable — synchronisation non lancée', 'err');
  }
  setTimeout(function () {
    if (b) b.disabled = false;
    if (c) c.disabled = false;
    if (lbl) lbl.textContent = 'Synchroniser';
  }, 3000);
  setTimeout(refresh, 1500);
}

export async function cancelSync() {
  let b = document.getElementById('bcancel');
  let lbl = document.getElementById('bcancel-lbl');
  b.disabled = true;
  lbl.textContent = 'Arrêt…';
  try {
    await fetch('/api/cancel', { method: 'POST' });
    toast('Arrêt de la synchronisation demandé', 'warn');
  } catch (e) {
    toast('Serveur injoignable', 'err');
  }
  setTimeout(function () {
    b.disabled = false;
    lbl.textContent = 'Arrêter';
  }, 3000);
  setTimeout(refresh, 1000);
}

export async function doResync() {
  let bAlert = document.getElementById('btn-alert-resync');
  let bSet = document.getElementById('btn-settings-resync');
  let c = document.getElementById('resync-confirm-btn');
  let prevAlert = bAlert ? bAlert.innerHTML : '';
  if (bAlert) {
    bAlert.disabled = true;
  }
  if (bSet) bSet.disabled = true;
  if (c) c.disabled = true;
  try {
    let r = await fetch('/api/resync', { method: 'POST' });
    let d = await r.json();
    if (d.ok) {
      toast('Resynchronisation (--resync) lancée', 'ok');
    } else {
      toast('Impossible de lancer la resynchronisation : ' + (d.error || 'erreur inconnue'), 'err');
    }
  } catch {
    toast('Serveur injoignable — resynchronisation non lancée', 'err');
  } finally {
    if (bAlert) {
      bAlert.disabled = false;
      bAlert.innerHTML = prevAlert;
    }
    if (bSet) bSet.disabled = false;
    if (c) c.disabled = false;
  }
  setTimeout(refresh, 1500);
}

export async function dismissNotice() {
  try {
    await fetch('/api/dismiss-notice', { method: 'POST' });
  } catch (e) {
    // Ignore error
  }
  refresh();
}
