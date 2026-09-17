import { colorizeLog } from './utils.js';

export {
  _originalSettings,
  getSettingsFromDOM,
  checkSettingsModified,
  openSettingsModal,
  closeSettingsModal,
  initSettingsModal,
  saveSettings
} from './settings.js';

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
