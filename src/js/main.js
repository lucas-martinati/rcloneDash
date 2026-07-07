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
   CTRL MAINTENU → « ouvrir le dossier »
   Bascule une classe sur <body> pour que le survol des
   liens de fichiers indique qu'on ouvrira le dossier parent.
   ═══════════════════════════════════════════════════ */
function updateCtrlState(e) {
  document.body.classList.toggle('folder-mode', e.ctrlKey || e.metaKey);
}
window.addEventListener('keydown', updateCtrlState);
window.addEventListener('keyup', updateCtrlState);
// Sécurité : on retire la classe si la fenêtre perd le focus (le keyup peut être manqué)
window.addEventListener('blur', function () { document.body.classList.remove('folder-mode'); });

/* ═══════════════════════════════════════════════════
   BOOT
   ═══════════════════════════════════════════════════ */
applyThemeIcon();
initDragAndDrop();
refresh();
_interval = setInterval(refresh, 10000);
setInterval(tickPulse, 1000);
