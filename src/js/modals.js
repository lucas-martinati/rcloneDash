import { toast } from './toasts.js';
import { colorizeLog } from './utils.js';

/* ═══════════════════════════════════════════════════
   LIMITE DE DÉBIT
   ═══════════════════════════════════════════════════ */
export function openBwModal() {
  document.getElementById('bw-modal').classList.add('show');
  fetch('/api/bwlimit').then(function (r) { return r.json(); }).then(function (d) {
    if (d.limit != null) document.getElementById('bw-select').value = d.limit;
  });
}

export function closeBwModal() { document.getElementById('bw-modal').classList.remove('show'); }

export async function saveBw() {
  var btn = document.getElementById('save-bw-btn');
  var limit = document.getElementById('bw-select').value;
  btn.disabled = true;
  btn.textContent = 'Application…';
  try {
    var r = await fetch('/api/bwlimit_save?limit=' + encodeURIComponent(limit));
    var d = await r.json();
    if (d.ok) {
      toast(limit
        ? 'Limite de débit appliquée — active dès la prochaine sync'
        : 'Limite de débit supprimée — vitesse maximale rétablie', 'ok');
      closeBwModal();
    } else {
      toast('Impossible d\'appliquer la limite : ' + d.error, 'err');
    }
  } catch (e) {
    toast('Serveur injoignable — limite non appliquée', 'err');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Appliquer la limite';
  }
}

/* ═══════════════════════════════════════════════════
   SIMULATION (dry run)
   ═══════════════════════════════════════════════════ */
export function openDryRunModal() { document.getElementById('dryrun-modal').classList.add('show'); }
export function closeDryRunModal() { document.getElementById('dryrun-modal').classList.remove('show'); }

export async function startDryRun() {
  var btn = document.getElementById('start-dryrun-btn');
  var out = document.getElementById('dryrun-output');
  btn.disabled = true;
  btn.textContent = 'Analyse en cours…';
  out.textContent = 'Analyse des différences entre le dossier local et Google Drive…\nCela peut prendre une à deux minutes.';

  try {
    var r = await fetch('/api/dryrun');
    var d = await r.json();
    if (d.ok) {
      out.innerHTML = colorizeLog(d.log || 'Aucun changement à appliquer : tout est déjà synchronisé.');
    } else {
      out.innerHTML = colorizeLog('La simulation a échoué :\n' + d.error);
    }
  } catch (e) {
    out.textContent = 'Serveur injoignable : ' + e.message;
  } finally {
    btn.disabled = false;
    btn.textContent = 'Relancer la simulation';
  }
}
