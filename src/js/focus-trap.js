/* ═══════════════════════════════════════════════════
   PIÈGE À FOCUS DES MODALES (accessibilité)
   Au clavier, une modale doit retenir le focus : Tab et
   Maj+Tab tournent en boucle dans la fenêtre, le focus
   part sur son premier élément à l'ouverture et revient
   sur le déclencheur à la fermeture. Aucune modification
   des fonctions openX : on observe la classe .show.
   ═══════════════════════════════════════════════════ */
const SEL = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

function focusables(modal) {
  return Array.from(modal.querySelectorAll(SEL))
    .filter(function (el) { return !el.disabled && el.offsetParent !== null; });
}

/* Modale « au-dessus » = la .show avec le plus grand z-index (les modales
   secondaires passent à 110), départage par ordre du DOM. */
function topModal() {
  const shown = Array.from(document.querySelectorAll('.modal-overlay.show'));
  if (!shown.length) return null;
  return shown.sort(function (a, b) {
    return (parseInt(getComputedStyle(a).zIndex, 10) || 0)
         - (parseInt(getComputedStyle(b).zIndex, 10) || 0);
  })[shown.length - 1];
}

export function initFocusTrap() {
  let lastFocused = null;

  document.addEventListener('keydown', function (e) {
    if (e.key !== 'Tab') return;
    const modal = topModal();
    if (!modal) return;
    const f = focusables(modal);
    if (!f.length) { e.preventDefault(); return; }
    const first = f[0], last = f[f.length - 1];
    const active = document.activeElement;
    if (!modal.contains(active)) {
      e.preventDefault();
      first.focus();
    } else if (e.shiftKey && active === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  });

  document.querySelectorAll('.modal-overlay').forEach(function (modal) {
    new MutationObserver(function () {
      const shown = modal.classList.contains('show');
      if (shown && !modal._trapped) {
        modal._trapped = true;
        lastFocused = document.activeElement;
        // .focus() est ignoré tant que la modale traverse sa transition
        // « visibility » (elle n'est pas encore focusable). On attend donc la
        // fin de transition, avec un filet de sécurité si elle n'a pas lieu
        // (mouvement réduit, transition absente).
        let done = false;
        const doFocus = function () {
          if (done || !modal.classList.contains('show')) return;
          done = true;
          const f = focusables(modal);
          (f[0] || modal).focus({ preventScroll: true });
        };
        modal.addEventListener('transitionend', doFocus, { once: true });
        setTimeout(doFocus, 250);
      } else if (!shown && modal._trapped) {
        modal._trapped = false;
        if (lastFocused && lastFocused.focus) lastFocused.focus({ preventScroll: true });
        lastFocused = null;
      }
    }).observe(modal, { attributes: true, attributeFilter: ['class'] });
  });
}
