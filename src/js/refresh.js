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
   ACTIONS SYNC
   ═══════════════════════════════════════════════════ */
export async function doSync() {
  let b = document.getElementById('bsync');
  let lbl = document.getElementById('bsync-lbl');
  b.disabled = true;
  lbl.textContent = 'Démarrage…';
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
    b.disabled = false;
    lbl.textContent = 'Synchroniser';
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
  if (
    !confirm(
      'Voulez-vous lancer une resynchronisation complète (--resync) ?\n\nCette opération reconstruit la base de comparaison locale et distante en conservant les fichiers les plus récents (--resync-mode newer).'
    )
  ) {
    return;
  }
  let bAlert = document.getElementById('btn-alert-resync');
  if (bAlert) {
    bAlert.disabled = true;
    bAlert.textContent = 'En cours…';
  }
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
      bAlert.textContent = 'Resynchroniser';
    }
  }
  setTimeout(refresh, 1500);
}

