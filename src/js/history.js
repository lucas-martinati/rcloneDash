import { S } from './state.js';
import { esc, fmtDT, fmtSize, colorizeLog } from './utils.js';
import { renderSparkline } from './sparkline.js';

/* ═══════════════════════════════════════════════════
   HISTORIQUE
   ═══════════════════════════════════════════════════ */
export function updateRuns(runs) {
  let sig = JSON.stringify(runs || []);
  if (sig === S.runsSig) return;
  S.runsSig = sig;

  window._errorLogsCache = {};
  let tb = document.getElementById('rtb');
  let em = document.getElementById('rem');
  if (!runs || !runs.length) {
    tb.innerHTML = '';
    em.style.display = '';
    document.getElementById('sparkline-wrap').innerHTML = '';
    return;
  }
  em.style.display = 'none';

  // Conserver les détails ouverts entre deux rafraîchissements
  let openSet = {};
  tb.querySelectorAll('tr.open').forEach(function (tr) { openSet[tr.dataset.start] = true; });

  let html = '';
  for (let i = 0; i < runs.length; i++) {
    let r = runs[i];
    let statusCls = r.status === 'success' ? 'ok' : r.status === 'failed' ? 'err' : 'run';
    let statusTxt = r.status === 'success' ? '✓ Réussie' : r.status === 'failed' ? '✗ Erreur' : '⟳ En cours';
    let isOpen = openSet[r.start];

    html += '<tr class="clickable' + (isOpen ? ' open' : '') + '" data-start="' + esc(r.start) + '"'
      + ' onclick="toggleRunDetails(' + i + ')" title="Afficher le détail de cette sync">'
      + '<td style="color:var(--muted)"><span class="chev">▶</span>' + fmtDT(r.start) + '</td>'
      + '<td><span class="pill ' + statusCls + '">' + statusTxt + '</span></td>'
      + '<td class="num' + (r.copied ? '' : ' dim') + '">' + (r.copied || 0) + '</td>'
      + '<td class="num' + (r.modified ? '' : ' dim') + '">' + (r.modified || 0) + '</td>'
      + '<td class="num' + (r.deleted ? '' : ' dim') + '">' + (r.deleted || 0) + '</td>'
      + '<td class="num" style="color:var(--muted)">' + (r.elapsed || '—') + '</td>'
      + '<td class="num" style="color:' + (r.errors > 0 ? 'var(--err)' : 'var(--faint)') + '">' + r.errors + '</td>'
      + '</tr>';

    let detHtml = '<div class="run-details-box">';
    detHtml += '<div class="run-details-header">'
      + '<span><b>Début :</b> ' + fmtDT(r.start) + '</span>'
      + '<span><b>Fin :</b> ' + fmtDT(r.end) + '</span>'
      + '<span><b>Durée :</b> ' + (r.elapsed || '—') + '</span>'
      + '</div>';

    if (r.error_logs && r.error_logs.length > 0) {
      window._errorLogsCache[i] = r.error_logs.join('\n');
      detHtml += '<div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:4px;">'
        + '<div style="color:var(--err);font-weight:600;">Erreurs :</div>'
        + '<button class="iconbtn" onclick="copyErrorLogs(' + i + ', this); event.stopPropagation();">Copier</button>'
        + '</div>'
        + '<div class="error-log-box">' + colorizeLog(r.error_logs.join('\n')) + '</div>';
    }

    if (r.synced_files && r.synced_files.length > 0) {
      detHtml += '<div style="font-weight:600;margin-top:10px">Fichiers affectés (' + r.synced_files.length + ') :</div>';
      detHtml += '<div class="run-details-files">';
      let labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
      for (let j = 0; j < Math.min(r.synced_files.length, 50); j++) {
        let f = r.synced_files[j];
        let cls = f.action || 'new';
        let pathArg = esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'");
        let actionCall = "openFile('" + pathArg + "', " + (cls === 'deleted' ? 'true' : 'false') + ', event)';
        let fSize = fmtSize(f.size);
        detHtml += '<div class="run-details-file recent-item file-link" onclick="event.stopPropagation(); ' + actionCall + '">'
          + '<span class="recent-dot ' + cls + '"></span>'
          + '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + '</span>'
          + '<span class="recent-path">' + esc(f.path) + '</span>'
          + (fSize ? '<span class="recent-size">' + fSize + '</span>' : '')
          + '</div>';
      }
      if (r.synced_files.length > 50) {
        detHtml += '<div style="padding:4px;color:var(--muted)">+ ' + (r.synced_files.length - 50) + ' autres fichiers…</div>';
      }
      detHtml += '</div>';
    } else {
      detHtml += '<div style="color:var(--faint);margin-top:10px">Aucun fichier n\'a changé durant cette sync.</div>';
    }
    detHtml += '</div>';

    html += '<tr id="run-det-' + i + '" style="display:' + (isOpen ? 'table-row' : 'none')
      + '"><td colspan="7" style="padding:0 10px 8px; max-width:0; white-space:normal;">' + detHtml + '</td></tr>';
  }
  tb.innerHTML = html;

  renderSparkline(runs);
}

export function toggleRunDetails(i) {
  let det = document.getElementById('run-det-' + i);
  if (!det) return;
  let open = det.style.display === 'none';
  det.style.display = open ? 'table-row' : 'none';
  let row = det.previousElementSibling;
  if (row) row.classList.toggle('open', open);
}

export function copyErrorLogs(idx, btn) {
  let text = window._errorLogsCache && window._errorLogsCache[idx];
  if (!text) return;
  navigator.clipboard.writeText(text).then(function () {
    btn.textContent = 'Copié ✓';
    setTimeout(function () { btn.textContent = 'Copier'; }, 2000);
  });
}
