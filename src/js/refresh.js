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
    // Le bloc live est piloté en temps réel par SSE (live-stream.js) ;
    // le poll n'y touche plus pour éviter tout scintillement.
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
