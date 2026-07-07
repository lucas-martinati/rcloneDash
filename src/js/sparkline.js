import { parseElapsed, fmtDT } from './utils.js';

/* ═══════════════════════════════════════════════════
   SPARKLINE — durées des syncs
   ═══════════════════════════════════════════════════ */
window._sparkTips = [];

export function renderSparkline(runs) {
  let wrap = document.getElementById('sparkline-wrap');
  let data = runs.slice(0, 24).reverse();
  if (data.length < 2) { wrap.innerHTML = ''; return; }

  let w = wrap.clientWidth || 300, h = 54, px = 6, py = 6;
  let values = data.map(function (r) { return parseElapsed(r.elapsed); });
  let maxV = Math.max.apply(null, values);
  if (maxV === 0) {
    values = data.map(function (r) { return r.files || 0; });
    maxV = Math.max.apply(null, values) || 1;
  }

  window._sparkTips = [];
  let slot = (w - px * 2) / data.length;
  let html = '<svg viewBox="0 0 ' + w + ' ' + h + '" style="width:100%;height:' + h + 'px;display:block" preserveAspectRatio="none" aria-label="Durée des dernières syncs">';

  for (let i = 0; i < data.length; i++) {
    let r = data[i];
    let v = values[i];
    let bw = Math.max(3, slot - 3);
    let bx = px + i * slot;
    let bh = Math.max(2, (v / maxV) * (h - py * 2));
    let by = h - py - bh;
    let color = r.status === 'failed' ? 'var(--err)' : (r.status === 'success' ? 'var(--ok)' : 'var(--run)');
    let st = r.status === 'failed' ? '✗ erreur' : (r.status === 'success' ? '✓ réussie' : '⟳ en cours');
    window._sparkTips.push(fmtDT(r.start) + ' — ' + (r.elapsed || '—') + ' · '
      + (r.files || 0) + ' fichier(s) · ' + st);

    html += '<rect class="chart-bar" x="' + bx + '" y="' + by + '" width="' + bw + '" height="' + bh
      + '" fill="' + color + '" rx="1.5"'
      + ' onmousemove="showTooltip(event, _sparkTips[' + i + '])"'
      + ' onmouseout="hideTooltip()"/>';
  }
  html += '</svg>';
  wrap.innerHTML = html;
  wrap.insertAdjacentHTML('beforeend', '<div id="chart-tt" class="chart-tooltip"></div>');
}

export function showTooltip(e, txt) {
  let tt = document.getElementById('chart-tt');
  if (!tt) return;
  tt.textContent = txt;
  tt.style.display = 'block';
  let x = e.clientX + 10;
  let y = e.clientY - 25;
  if (x + tt.offsetWidth > window.innerWidth - 4) x = e.clientX - tt.offsetWidth - 10;
  if (y < 4) y = e.clientY + 15;
  tt.style.left = x + 'px';
  tt.style.top = y + 'px';
}

export function hideTooltip() {
  let tt = document.getElementById('chart-tt');
  if (tt) tt.style.display = 'none';
}
