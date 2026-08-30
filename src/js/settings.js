import { toast } from './toasts.js';

/* ═══════════════════════════════════════════════════
   CONFIGURATION (PARAMÈTRES)
   ═══════════════════════════════════════════════════ */
export let _originalSettings = null;

export function getSettingsFromDOM() {
  return {
    remote: document.getElementById('set-remote')?.value || '',
    local_dir: document.getElementById('set-local-dir')?.value || '',
    timer_interval: document.getElementById('set-timer')?.value || '',
    bwlimit: document.getElementById('set-bwlimit')?.value || ''
  };
}

export function checkSettingsModified() {
  let btn = document.getElementById('btn-save-settings');
  if (!btn) return;

  let current = getSettingsFromDOM();
  let isValid = current.remote.trim() !== '' && current.local_dir.trim() !== '';

  if (!_originalSettings) {
    // Si l'état initial n'a pas encore été figé, on l'initialise avec les valeurs actuelles
    _originalSettings = { ...current };
    btn.disabled = true;
    return;
  }

  let isDirty =
    current.remote !== _originalSettings.remote ||
    current.local_dir !== _originalSettings.local_dir ||
    current.timer_interval !== _originalSettings.timer_interval ||
    current.bwlimit !== _originalSettings.bwlimit;

  btn.disabled = !(isDirty && isValid);
}

export function setSettingsInputsDisabled(disabled) {
  ['set-remote', 'set-local-dir', 'set-timer', 'set-bwlimit'].forEach(id => {
    let el = document.getElementById(id);
    if (el) el.disabled = disabled;
  });
}

export function openSettingsModal() {
  let modal = document.getElementById('settings-modal');
  if (modal) modal.classList.add('show');

  let btn = document.getElementById('btn-save-settings');
  if (btn) btn.disabled = true;

  setSettingsInputsDisabled(true);

  fetch('/api/settings')
    .then(r => r.json())
    .then(d => {
      if (d.remote != null) document.getElementById('set-remote').value = d.remote;
      if (d.local_dir != null) document.getElementById('set-local-dir').value = d.local_dir;
      if (d.timer_interval != null) document.getElementById('set-timer').value = d.timer_interval;
      if (d.bwlimit != null) document.getElementById('set-bwlimit').value = d.bwlimit;

      _originalSettings = getSettingsFromDOM();
      setSettingsInputsDisabled(false);
      checkSettingsModified();
    })
    .catch(() => {
      _originalSettings = getSettingsFromDOM();
      setSettingsInputsDisabled(false);
      checkSettingsModified();
    });
}

export function closeSettingsModal() {
  let modal = document.getElementById('settings-modal');
  if (modal) modal.classList.remove('show');
  setSettingsInputsDisabled(false);
}

export function initSettingsModal() {
  let modal = document.getElementById('settings-modal');
  if (!modal) return;
  // Écouteurs délégués centralisés pour tous les types d'événements de saisie
  modal.addEventListener('input', checkSettingsModified);
  modal.addEventListener('change', checkSettingsModified);
  modal.addEventListener('keyup', checkSettingsModified);
  modal.addEventListener('paste', () => setTimeout(checkSettingsModified, 0));
}

export async function saveSettings() {
  let btn = document.getElementById('btn-save-settings');
  let current = getSettingsFromDOM();
  let data = {
    remote: current.remote.trim(),
    local_dir: current.local_dir.trim(),
    timer_interval: current.timer_interval,
    bwlimit: current.bwlimit
  };

  if (!data.remote || !data.local_dir) {
    toast('La cible et le dossier local sont requis.', 'err');
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
      _originalSettings = getSettingsFromDOM();
      checkSettingsModified();
      toast('Paramètres appliqués. Redémarrage du Dashboard...', 'ok');
      setTimeout(() => {
        window.location.reload();
      }, 2000);
    } else {
      toast("Impossible d'appliquer : " + d.error, 'err');
      btn.innerHTML =
        '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path><polyline points="17 21 17 13 7 13 7 21"></polyline><polyline points="7 3 7 8 15 8"></polyline></svg> Enregistrer & Redémarrer';
      checkSettingsModified();
    }
  } catch (e) {
    // Expected because the server is restarting itself!
    _originalSettings = getSettingsFromDOM();
    checkSettingsModified();
    toast('Paramètres appliqués. Redémarrage du Dashboard...', 'ok');
    setTimeout(() => {
      window.location.reload();
    }, 2000);
  }
}
