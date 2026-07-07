/* ═══════════════════════════════════════════════════
   SPARKLINE — durées des syncs
   ═══════════════════════════════════════════════════ */
window._sparkTips = [];

function renderSparkline(runs) {
  var wrap = document.getElementById('sparkline-wrap');
  var data = runs.slice(0, 24).reverse();
  if (data.length < 2) { wrap.innerHTML = ''; return; }

  var w = wrap.clientWidth || 300, h = 54, px = 6, py = 6;
  var values = data.map(function (r) { return parseElapsed(r.elapsed); });
  var maxV = Math.max.apply(null, values);
  if (maxV === 0) {
    values = data.map(function (r) { return r.files || 0; });
    maxV = Math.max.apply(null, values) || 1;
  }

  window._sparkTips = [];
  var slot = (w - px * 2) / data.length;
  var html = '<svg viewBox="0 0 ' + w + ' ' + h + '" style="width:100%;height:' + h + 'px;display:block" preserveAspectRatio="none" aria-label="Durée des dernières syncs">';

  for (var i = 0; i < data.length; i++) {
    var r = data[i];
    var v = values[i];
    var bw = Math.max(3, slot - 3);
    var bx = px + i * slot;
    var bh = Math.max(2, (v / maxV) * (h - py * 2));
    var by = h - py - bh;
    var color = r.status === 'failed' ? 'var(--err)' : (r.status === 'success' ? 'var(--ok)' : 'var(--run)');
    var st = r.status === 'failed' ? '✗ erreur' : (r.status === 'success' ? '✓ réussie' : '⟳ en cours');
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

function showTooltip(e, txt) {
  var tt = document.getElementById('chart-tt');
  if (!tt) return;
  tt.textContent = txt;
  tt.style.display = 'block';
  var x = e.clientX + 10;
  var y = e.clientY - 25;
  if (x + tt.offsetWidth > window.innerWidth - 4) x = e.clientX - tt.offsetWidth - 10;
  if (y < 4) y = e.clientY + 15;
  tt.style.left = x + 'px';
  tt.style.top = y + 'px';
}

function hideTooltip() {
  var tt = document.getElementById('chart-tt');
  if (tt) tt.style.display = 'none';
}
