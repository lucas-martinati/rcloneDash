import { toast } from './toasts.js';
import { colorizeLog } from './utils.js';

/* ═══════════════════════════════════════════════════
   CONFIGURATION (PARAMÈTRES)
   ═══════════════════════════════════════════════════ */
export function openSettingsModal() {
  document.getElementById('settings-modal').classList.add('show');

  fetch('/api/settings')
    .then(r => r.json())
    .then(d => {
      if (d.remote != null) document.getElementById('set-remote').value = d.remote;
      if (d.local_dir != null) document.getElementById('set-local-dir').value = d.local_dir;
      if (d.timer_interval != null) document.getElementById('set-timer').value = d.timer_interval;
      if (d.bwlimit != null) document.getElementById('set-bwlimit').value = d.bwlimit;
    });
}

export function closeSettingsModal() {
  document.getElementById('settings-modal').classList.remove('show');
}

export async function saveSettings() {
  let btn = document.getElementById('btn-save-settings');
  let data = {
    remote: document.getElementById('set-remote').value.trim(),
    local_dir: document.getElementById('set-local-dir').value.trim(),
    timer_interval: document.getElementById('set-timer').value,
    bwlimit: document.getElementById('set-bwlimit').value
  };
  
  if (!data.remote || !data.local_dir) {
    toast("La cible et le dossier local sont requis.", "err");
    return;
  }
  
  btn.disabled = true;
  btn.innerHTML = 'Enregistrement...';
  
  try {
    let r = await fetch('/api/settings_save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data)
    });
    let d = await r.json();
    if (d.ok) {
      toast('Paramètres appliqués. Redémarrage du Dashboard...', 'ok');
      setTimeout(() => {
        window.location.reload();
      }, 2000);
    } else {
      toast("Impossible d'appliquer : " + d.error, 'err');
      btn.disabled = false;
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path><polyline points="17 21 17 13 7 13 7 21"></polyline><polyline points="7 3 7 8 15 8"></polyline></svg> Enregistrer & Redémarrer';
    }
  } catch (e) {
    // Expected because the server is restarting itself!
    toast('Paramètres appliqués. Redémarrage du Dashboard...', 'ok');
    setTimeout(() => {
      window.location.reload();
    }, 2000);
  }
}

/* ═══════════════════════════════════════════════════
   SIMULATION (dry run)
   ═══════════════════════════════════════════════════ */
export function openDryRunModal() {
  document.getElementById('dryrun-modal').classList.add('show');
}
export function closeDryRunModal() {
  document.getElementById('dryrun-modal').classList.remove('show');
}

export async function startDryRun() {
  let btn = document.getElementById('start-dryrun-btn');
  let out = document.getElementById('dryrun-output');
  btn.disabled = true;
  btn.textContent = 'Analyse en cours…';
  out.classList.remove('is-empty');
  out.textContent =
    'Analyse des différences entre le dossier local et Google Drive…\nCela peut prendre une à deux minutes.';

  try {
    let r = await fetch('/api/dryrun');
    let d = await r.json();
    if (d.ok) {
      out.innerHTML = colorizeLog(
        d.log || 'Aucun changement à appliquer : tout est déjà synchronisé.'
      );
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
