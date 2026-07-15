/* ═══════════════════════════════════════════════════
   TOASTS
   ═══════════════════════════════════════════════════ */
export function toast(msg, type) {
  let c = document.getElementById('toasts');
  let t = document.createElement('div');
  t.className = 'toast ' + (type || '');
  t.textContent = msg;
  c.appendChild(t);
  requestAnimationFrame(function () {
    t.classList.add('in');
  });
  setTimeout(function () {
    t.classList.remove('in');
    setTimeout(function () {
      t.remove();
    }, 300);
  }, 4200);
}
