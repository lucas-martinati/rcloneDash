/* ═══════════════════════════════════════════════════
   STATE
   ═══════════════════════════════════════════════════ */
let _theme = 'dark';
let _llc = 0;
let _interval = null;
let _curState = '';

/* ═══════════════════════════════════════════════════
   UTILS
   ═══════════════════════════════════════════════════ */
function esc(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function fmtT(ts) {
  if (!ts) return '—';
  try {
    return new Date(ts).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return ts.slice(11, 19) || ts;
  }
}

function fmtDT(ts) {
  if (!ts) return '—';
  try {
    return new Date(ts).toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' });
  } catch {
    return ts.slice(0, 16);
  }
}

function spin(v) { document.getElementById('spin').classList.toggle('on', v); }

/* ═══════════════════════════════════════════════════
   THEME TOGGLE
   ═══════════════════════════════════════════════════ */
function toggleTheme() {
  _theme = _theme === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = _theme;
  document.getElementById('ti').innerHTML = _theme === 'dark'
    ? '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>'
    : '<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>';
}

/* ═══════════════════════════════════════════════════
   STATUS BADGE
   ═══════════════════════════════════════════════════ */
function badge(state) {
  var b = document.getElementById('sbadge');
  var l = document.getElementById('slbl');
  var d = document.getElementById('sdot');
  var map = {
    running: ['run', 'En cours…'],
    success: ['ok', 'Succès'],
    failed: ['err', 'Erreur'],
    idle: ['idle', 'En attente']
  };
  var info = map[state] || ['idle', state];
  b.className = 'badge ' + info[0];
  l.textContent = info[1];
  d.className = 'dot' + (state === 'running' ? ' pulse' : '');
  
  var bsync = document.getElementById('bsync');
  var bcancel = document.getElementById('bcancel');
  if (state === 'running') {
    if (bsync) bsync.style.display = 'none';
    if (bcancel) bcancel.style.display = 'inline-flex';
  } else {
    if (bsync) bsync.style.display = 'inline-flex';
    if (bcancel) bcancel.style.display = 'none';
  }
}

/* ═══════════════════════════════════════════════════
   SMART AUTO-REFRESH
   ═══════════════════════════════════════════════════ */
function setSmartRefresh(state) {
  if (state === _curState) return;
  _curState = state;
  if (_interval) clearInterval(_interval);
  var ms = state === 'running' ? 3000 : state === 'failed' ? 5000 : 10000;
  _interval = setInterval(refresh, ms);
}

/* ═══════════════════════════════════════════════════
   ALERTS
   ═══════════════════════════════════════════════════ */
function updateAlerts(data) {
  if (data.quota) {
      if (data.quota.error) {
          document.getElementById('quota-text').textContent = 'Erreur : ' + data.quota.error;
          document.getElementById('quota-text').style.color = 'var(--err)';
      } else {
          const formatBytes = (bytes) => (bytes / (1024 ** 3)).toFixed(1) + ' Go';
          const used = data.quota.used || 0;
          const total = data.quota.total || 1;
          const percent = Math.min(100, Math.round((used / total) * 100));
          
          document.getElementById('quota-text').textContent = `${formatBytes(used)} / ${formatBytes(total)} (${percent}%)`;
          document.getElementById('quota-text').style.color = 'var(--text)';
          document.getElementById('quota-bar').style.width = percent + '%';
          
          if (percent > 90) {
              document.getElementById('quota-bar').style.background = 'var(--err)';
          } else {
              document.getElementById('quota-bar').style.background = 'var(--primary)';
          }
      }
  }

  var ban = document.getElementById('alert-banner');
  var msg = document.getElementById('alert-msg');
  var kpis = data.kpis;
  var disk = data.disk;
  var live = data.live;

  if (kpis.consecutive_failures >= 2) {
    ban.className = 'alert-banner show-err';
    msg.textContent = kpis.consecutive_failures + ' syncs consécutives en erreur — ' + (kpis.last_error_msg || 'voir les logs pour détails');
  } else if (live && live.is_syncing && live.duration_s > 300) {
    ban.className = 'alert-banner show-warn';
    msg.textContent = 'Sync lente — en cours depuis ' + Math.floor(live.duration_s / 60) + ' min ' + (live.duration_s % 60) + 's';
  } else if (disk.pct > 90) {
    ban.className = 'alert-banner show-err';
    msg.textContent = 'Disque local à ' + disk.pct + '% — espace disque critique !';
  } else {
    ban.className = 'alert-banner';
  }

  // Slow sync badge
  var sb = document.getElementById('slow-badge');
  sb.style.display = (live && live.is_syncing && live.duration_s > 300) ? '' : 'none';
}

/* ═══════════════════════════════════════════════════
   KPIs
   ═══════════════════════════════════════════════════ */
function updateKPIs(data) {
  var svc = data.service, timer = data.timer, disk = data.disk;
  var runs = data.runs || [], kpis = data.kpis;

  // Row 1
  document.getElementById('kt').textContent = timer.active ? 'Actif' : 'Inactif';
  document.getElementById('kts').textContent = (timer.next_run || 'toutes les 5 min').slice(0, 50);

  var last = runs.find(function(r) { return r.status !== 'running'; });
  document.getElementById('kl').textContent = last ? fmtT(last.start) : '—';
  document.getElementById('kls').textContent = last
    ? (last.status === 'success' ? '✓ Succès' : '✗ Erreur') : '—';
  document.getElementById('kd').textContent = svc.duration !== '—'
    ? svc.duration : (last && last.elapsed ? last.elapsed : '—');

  // Disk
  var dkpi = document.getElementById('disk-kpi');
  var dkEl = document.getElementById('kdk');
  document.getElementById('kdk').textContent = disk.used + ' GB';
  document.getElementById('kdks').textContent = disk.free + ' GB libres / ' + disk.total + ' GB';
  document.getElementById('dfill').style.width = disk.pct + '%';
  var diskDanger = disk.pct > 90;
  document.getElementById('dfill').classList.toggle('danger', diskDanger);
  dkpi.classList.toggle('danger', diskDanger);
  dkEl.style.color = diskDanger ? 'var(--err)' : '';

  // Syncs today
  var today = new Date().toISOString().slice(0, 10);
  var td = runs.filter(function(r) { return r.start && r.start.startsWith(today); });
  var tok = td.filter(function(r) { return r.status === 'success'; }).length;
  var terr = td.filter(function(r) { return r.status === 'failed'; }).length;
  document.getElementById('kr').textContent = td.length;
  document.getElementById('krs').textContent = tok + ' OK' + (terr ? ' · ' + terr + ' erreur(s)' : '');

  // Row 2
  document.getElementById('kf').textContent = kpis.total_files > 0
    ? kpis.total_files.toLocaleString('fr-FR') : '—';
  document.getElementById('ksp').textContent = kpis.avg_speed || '—';

  var kcEl = document.getElementById('kc');
  kcEl.textContent = kpis.conflicts_today;
  kcEl.style.color = kpis.conflicts_today > 0 ? 'var(--warn)' : '';
  document.getElementById('conflict-kpi').classList.toggle('danger', kpis.conflicts_today > 0);

  var rateVal = kpis.success_rate_7d;
  document.getElementById('ksr').textContent = rateVal + '%';
  document.getElementById('ksr').style.color = rateVal < 90 ? 'var(--err)' : rateVal < 100 ? 'var(--warn)' : '';
  document.getElementById('srfill').style.width = rateVal + '%';
  document.getElementById('srfill').classList.toggle('danger', rateVal < 90);
}

/* ═══════════════════════════════════════════════════
   LIVE SYNC SECTION
   ═══════════════════════════════════════════════════ */
function updateLive(live) {
  var section = document.getElementById('live-section');
  if (!live || (!live.is_syncing && live.phase_index <= 0)) {
    section.classList.remove('active');
    return;
  }
  section.classList.add('active');

  // Phase stepper
  var phaseNames = ['Listings', 'Path1 diffs', 'Path2 diffs', 'Applying', 'Updating', 'Done'];
  var stepper = document.getElementById('phase-stepper');
  var html = '';
  for (var i = 0; i < phaseNames.length; i++) {
    var cls = '', icon = '○';
    if (i < live.phase_index) { cls = 'done'; icon = '✓'; }
    else if (i === live.phase_index) { cls = 'current'; icon = '●'; }
    if (i > 0) html += '<span class="phase-arrow">→</span>';
    html += '<span class="phase-step ' + cls + '">' + icon + ' ' + phaseNames[i] + '</span>';
  }
  stepper.innerHTML = html;

  // Elapsed
  var elapsed = document.getElementById('live-elapsed');
  if (live.duration_s > 0) {
    var m = Math.floor(live.duration_s / 60);
    var s = live.duration_s % 60;
    elapsed.textContent = (m > 0 ? m + 'min ' : '') + s + 's';
  } else {
    elapsed.textContent = (live.transfer && live.transfer.elapsed) || '';
  }

  // Transfer stats
  var t = live.transfer || {};
  document.getElementById('tf-done').textContent = t.done || '—';
  document.getElementById('tf-total').textContent = t.total || '—';
  document.getElementById('tf-pct').textContent = t.pct != null ? t.pct + '%' : '—';
  document.getElementById('tf-speed').textContent = t.speed || '—';
  document.getElementById('tf-eta').textContent = t.eta || '—';
  document.getElementById('tf-checks').textContent =
    t.checks_done != null ? t.checks_done + ' / ' + t.checks_total : '—';
  document.getElementById('tf-bar').style.width = (t.pct || 0) + '%';

  // Active files
  var aw = document.getElementById('active-wrap');
  if (live.active_files && live.active_files.length > 0) {
    aw.style.display = '';
    var html = '<div class="kl">' + (live.active_files.length > 1 ? 'Fichiers en cours de transfert' : 'Fichier en cours de transfert') + '</div>';
    for (var i = 0; i < live.active_files.length; i++) {
      var f = live.active_files[i];
      html += '<div style="margin-top: 6px; margin-bottom: 8px;">'
        + '<div class="active-fname" title="' + esc(f.name) + '">' + esc(f.name) + ' (' + f.pct + '%)</div>'
        + '<div class="tf-pbar"><div class="tf-pfill" style="width:' + f.pct + '%"></div></div>'
        + '</div>';
    }
    aw.innerHTML = html;
  } else if (live.active_file) {
    aw.style.display = '';
    aw.innerHTML = '<div class="kl">Fichier en cours de transfert</div>'
      + '<div class="active-fname" id="active-file">' + esc(live.active_file) + '</div>'
      + '<div class="tf-pbar"><div class="tf-pfill" id="active-bar" style="width:' + live.active_file_pct + '%"></div></div>';
  } else {
    aw.style.display = 'none';
  }

  // Synced files during active run
  var lsw = document.getElementById('live-synced-wrap');
  var lsl = document.getElementById('live-synced-list');
  if (live.synced_files && live.synced_files.length > 0) {
    lsw.style.display = '';
    var labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
    var html = '';
    for (var i = live.synced_files.length - 1; i >= 0; i--) {
      var f = live.synced_files[i];
      var cls = f.action || 'new';
      var actionCall = cls === 'deleted' 
        ? `openFile('${esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}', true)` 
        : `openFile('${esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}')`;
      html += '<div class="recent-item" style="padding: 4px 0; border-bottom: 1px solid var(--border);" onclick="' + actionCall + '">'
        + '<span class="recent-dot ' + cls + '"></span>'
        + '<span class="recent-label ' + cls + '" style="font-size: 9px; padding: 1px 4px; min-width: 45px;">' + (labels[cls] || cls) + '</span>'
        + '<span class="recent-path" style="font-size: 11px; font-family:\'Courier New\',monospace; color:var(--text);" title="' + esc(f.path) + '">' + esc(f.path) + '</span>'
        + '<span class="recent-time" style="font-size: 10px;">' + esc(f.time) + '</span>'
        + '</div>';
    }
    lsl.innerHTML = html;
  } else {
    lsw.style.display = 'none';
  }

  // Changes
  renderChanges('ch-p1', live.changes.path1);
  renderChanges('ch-p2', live.changes.path2);
}

function renderChanges(id, ch) {
  var el = document.getElementById(id);
  var items = [];
  var arr;
  arr = ch['new'] || [];
  for (var i = 0; i < arr.length; i++)
    items.push('<div class="change-item"><span class="change-dot new"></span>' + esc(arr[i]) + '</div>');
  arr = ch.modified || [];
  for (var i = 0; i < arr.length; i++)
    items.push('<div class="change-item"><span class="change-dot modified"></span>' + esc(arr[i]) + '</div>');
  arr = ch.deleted || [];
  for (var i = 0; i < arr.length; i++)
    items.push('<div class="change-item"><span class="change-dot deleted"></span>' + esc(arr[i]) + '</div>');
  el.innerHTML = items.length
    ? items.join('')
    : '<span style="color:var(--faint);font-size:11px">Aucun changement</span>';
}

/* ═══════════════════════════════════════════════════
   CHART SVG
   ═══════════════════════════════════════════════════ */
function renderSparkline(runs) {
  var wrap = document.getElementById('sparkline-wrap');
  var data = runs.slice(0, 20).reverse();
  if (data.length < 2) { wrap.innerHTML = ''; return; }

  var w = wrap.clientWidth || 300, h = 60, px = 8, py = 8;
  var maxFiles = Math.max.apply(null, data.map(function(r) { return r.files || 0; }));
  if (maxFiles === 0) maxFiles = 1;

  var barWidth = (w - px * 2) / data.length - 2;
  var html = '<svg viewBox="0 0 ' + w + ' ' + h + '" style="width:100%;height:' + h + 'px;display:block" preserveAspectRatio="none">';

  for (var i = 0; i < data.length; i++) {
    var r = data[i];
    var v = r.files || 0;
    var bw = Math.max(2, barWidth);
    var bx = px + i * (w - px * 2) / data.length;
    var bh = Math.max(2, (v / maxFiles) * (h - py * 2));
    var by = h - py - bh;
    
    var color = r.status === 'failed' ? 'var(--err)' : (r.status === 'success' ? 'var(--ok)' : 'var(--run)');
    var tt = esc(fmtT(r.start)) + ' : ' + v + ' fichier(s) [' + r.status + ']';
    
    html += '<rect class="chart-bar" x="' + bx + '" y="' + by + '" width="' + bw + '" height="' + bh + '" fill="' + color + '" rx="1"'
      + ' onmousemove="showTooltip(event, \'' + tt + '\')"'
      + ' onmouseout="hideTooltip()"/>';
  }
  html += '</svg>';
  if (!document.getElementById('chart-tt')) {
    wrap.insertAdjacentHTML('beforeend', '<div id="chart-tt" class="chart-tooltip"></div>');
  }
  var oldTt = document.getElementById('chart-tt');
  wrap.innerHTML = html;
  wrap.appendChild(oldTt || document.createElement('div'));
  document.getElementById('chart-tt').className = 'chart-tooltip';
}
function showTooltip(e, txt) {
  var tt = document.getElementById('chart-tt');
  if(!tt) return;
  tt.textContent = txt;
  tt.style.display = 'block';
  var wrap = document.getElementById('sparkline-wrap').getBoundingClientRect();
  tt.style.left = (e.clientX - wrap.left + 10) + 'px';
  tt.style.top = (e.clientY - wrap.top - 25) + 'px';
}
function hideTooltip() {
  var tt = document.getElementById('chart-tt');
  if(tt) tt.style.display = 'none';
}

/* ═══════════════════════════════════════════════════
   RUNS TABLE
   ═══════════════════════════════════════════════════ */
function updateRuns(runs) {
  var tb = document.getElementById('rtb');
  var em = document.getElementById('rem');
  if (!runs || !runs.length) {
    tb.innerHTML = '';
    em.style.display = '';
    document.getElementById('sparkline-wrap').innerHTML = '';
    return;
  }
  em.style.display = 'none';

  var html = '';
  for (var i = 0; i < runs.length; i++) {
    var r = runs[i];
    var statusCls = r.status === 'success' ? 'ok' : r.status === 'failed' ? 'err' : 'run';
    var statusTxt = r.status === 'success' ? '✓ OK' : r.status === 'failed' ? '✗ Erreur' : '⟳ En cours';
    
    var trCls = ' class="clickable" onclick="toggleRunDetails(' + i + ')" title="Voir les détails"';
    
    html += '<tr' + trCls + '>'
      + '<td style="color:var(--muted)">' + fmtDT(r.start) + '</td>'
      + '<td><span class="pill ' + statusCls + '">' + statusTxt + '</span></td>'
      + '<td>' + (r.copied || 0) + '</td>'
      + '<td>' + (r.modified || 0) + '</td>'
      + '<td>' + (r.deleted || 0) + '</td>'
      + '<td style="color:var(--muted)">' + (r.elapsed || '—') + '</td>'
      + '<td style="color:' + (r.errors > 0 ? 'var(--err)' : 'var(--muted)') + '">' + r.errors + '</td>'
      + '</tr>';
      
    var detHtml = '<div class="run-details-box">';
    detHtml += '<div class="run-details-header">'
      + '<span><b>Début:</b> ' + fmtDT(r.start) + '</span>'
      + '<span><b>Fin:</b> ' + fmtDT(r.end) + '</span>'
      + '<span><b>Durée:</b> ' + (r.elapsed || '—') + '</span>'
      + '</div>';
    
    if (r.error_logs && r.error_logs.length > 0) {
      detHtml += '<div style="color:var(--err);font-weight:bold;margin-bottom:4px">Logs d\'erreurs :</div>';
      detHtml += '<div class="error-log-box" style="margin-top:0">' + esc(r.error_logs.join('\n')) + '</div>';
    }
    
    if (r.synced_files && r.synced_files.length > 0) {
      detHtml += '<div style="font-weight:bold;margin-top:10px">Fichiers affectés :</div>';
      detHtml += '<div class="run-details-files">';
      var labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
      for (var j = 0; j < Math.min(r.synced_files.length, 50); j++) {
        var f = r.synced_files[j];
        var cls = f.action || 'new';
        var actionCall = cls === 'deleted' 
          ? `openFile('${esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}', true)` 
          : `openFile('${esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}')`;
          
        detHtml += '<div class="run-details-file recent-item" style="padding:4px" onclick="' + actionCall + '">'
          + '<span class="recent-dot ' + cls + '"></span>'
          + '<span class="recent-label ' + cls + '" style="font-size: 9px; padding: 1px 4px; min-width: 45px;">' + (labels[cls] || cls) + '</span>'
          + '<span class="recent-path" style="font-size: 11px; font-family:\'Courier New\',monospace; color:var(--text);">' + esc(f.path) + '</span>'
          + '</div>';
      }
      if(r.synced_files.length > 50) detHtml += '<div style="padding:4px;color:var(--muted)">+ ' + (r.synced_files.length - 50) + ' autres fichiers...</div>';
      detHtml += '</div>';
    } else {
      detHtml += '<div style="color:var(--faint);margin-top:10px">Aucun fichier synchronisé durant cette exécution.</div>';
    }
    detHtml += '</div>';
    
    html += '<tr id="run-det-' + i + '" style="display:none"><td colspan="7" style="padding:0 8px 8px">' + detHtml + '</td></tr>';
  }
  tb.innerHTML = html;

  // Sparkline
  renderSparkline(runs);
}
function toggleRunDetails(i) {
  var el = document.getElementById('run-det-' + i);
  if(el) el.style.display = el.style.display === 'none' ? 'table-row' : 'none';
}

/* ═══════════════════════════════════════════════════
   LOGS
   ═══════════════════════════════════════════════════ */
function updateLogs(logs) {
  var w = document.getElementById('lwrap');
  var sc = document.getElementById('lscroll');
  var atBot = sc.scrollHeight - sc.scrollTop - sc.clientHeight < 50;
  if (logs.length !== _llc) {
    var html = '';
    for (var i = 0; i < logs.length; i++) {
      html += '<div class="ll ' + logs[i].l + '">' + esc(logs[i].t) + '</div>';
    }
    w.innerHTML = html;
    _llc = logs.length;
    if (atBot) sc.scrollTop = sc.scrollHeight;
  }
}

function logsBot() {
  var s = document.getElementById('lscroll');
  s.scrollTop = s.scrollHeight;
}

/* ═══════════════════════════════════════════════════
   RECENT FILES
   ═══════════════════════════════════════════════════ */
function updateRecentFiles(files) {
  var list = document.getElementById('recent-list');
  var countEl = document.getElementById('recent-count');

  if (!files || !files.length) {
    list.innerHTML = '<div class="empty">Aucun fichier récent dans les logs</div>';
    countEl.textContent = '';
    return;
  }

  countEl.textContent = files.length + ' fichier(s)';
  var labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
  var reversed = files.slice().reverse();
  var html = '';

  for (var i = 0; i < reversed.length; i++) {
    var f = reversed[i];
    var cls = f.action || 'new';
    var actionCall = cls === 'deleted' 
      ? `openFile('${esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}', true)` 
      : `openFile('${esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}')`;
    html += '<div class="recent-item" onclick="' + actionCall + '">'
      + '<span class="recent-dot ' + cls + '"></span>'
      + '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + '</span>'
      + '<span class="recent-path" title="' + esc(f.path) + '">' + esc(f.path) + '</span>'
      + '<span class="recent-time">' + fmtT(f.time) + '</span>'
      + '</div>';
  }
  list.innerHTML = html;
}

function filterRecent() {
  var q = document.getElementById('recent-search').value.toLowerCase();
  var items = document.querySelectorAll('#recent-list .recent-item');
  for (var i = 0; i < items.length; i++) {
    var path = items[i].querySelector('.recent-path').textContent.toLowerCase();
    items[i].style.display = path.indexOf(q) !== -1 ? '' : 'none';
  }
}

/* ═══════════════════════════════════════════════════
   MAIN REFRESH
   ═══════════════════════════════════════════════════ */
async function refresh() {
  spin(true);
  try {
    var r = await fetch('/api/status');
    if (!r.ok) throw new Error(r.status);
    var d = await r.json();

    badge(d.service.state);
    setSmartRefresh(d.service.state);
    updateAlerts(d);
    updateKPIs(d);
    updateLive(d.live);
    updateRuns(d.runs);
    updateLogs(d.logs);
    updateRecentFiles(d.recent_files);
    document.getElementById('ts').textContent = 'MàJ ' + fmtT(d.ts);
  } catch (e) {
    document.getElementById('ts').textContent = '⚠ Serveur injoignable';
  } finally {
    spin(false);
  }
}

/* ═══════════════════════════════════════════════════
   MANUAL SYNC TRIGGER
   ═══════════════════════════════════════════════════ */
async function doSync() {
  var b = document.getElementById('bsync');
  b.disabled = true;
  b.textContent = 'Démarrage…';
  try {
    var r = await fetch('/api/trigger');
    var d = await r.json();
    b.textContent = d.ok ? '✓ Lancée !' : '✗ ' + (d.error || 'Erreur');
  } catch {
    b.textContent = '✗ Erreur';
  }
  setTimeout(function() {
    b.disabled = false;
    b.innerHTML = '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg> Sync';
  }, 3000);
  setTimeout(refresh, 1500);
}

async function cancelSync() {
  var b = document.getElementById('bcancel');
  b.disabled = true;
  b.textContent = 'Annulation…';
  try {
    await fetch('/api/cancel');
  } catch (e) {
    console.error(e);
  }
  setTimeout(function() {
    b.disabled = false;
    b.innerHTML = '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg> Annuler';
  }, 3000);
  setTimeout(refresh, 1000);
}

/* ═══════════════════════════════════════════════════
   TREE VIEWER & FILE OPEN
   ═══════════════════════════════════════════════════ */
let _currentTreeDir = '';
function openTreeModal() {
  document.getElementById('tree-modal').classList.add('show');
  loadTree('');
}
function closeTreeModal() {
  document.getElementById('tree-modal').classList.remove('show');
}
function treeUp() {
  if (!_currentTreeDir) return;
  var parts = _currentTreeDir.split('/');
  parts.pop();
  loadTree(parts.join('/'));
}
async function loadTree(dir) {
  var list = document.getElementById('tree-list');
  var pathEl = document.getElementById('tree-path');
  var upBtn = document.getElementById('tree-up-btn');
  list.innerHTML = '<div style="padding:20px;text-align:center;color:var(--faint)">Chargement...</div>';
  try {
    var r = await fetch('/api/tree?dir=' + encodeURIComponent(dir));
    var d = await r.json();
    if (d.error) throw new Error(d.error);
    _currentTreeDir = d.current_dir;
    pathEl.textContent = '/' + (_currentTreeDir || '');
    upBtn.disabled = !_currentTreeDir;
    
    if (!d.items || d.items.length === 0) {
      list.innerHTML = '<div class="empty">Dossier vide</div>';
      return;
    }
    var html = '';
    for (var i=0; i<d.items.length; i++) {
      var item = d.items[i];
      var icon = item.is_dir ? '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>' : '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>';
      var sizeTxt = item.is_dir ? '' : (item.size > 1024*1024 ? (item.size/(1024*1024)).toFixed(1)+' MB' : (item.size/1024).toFixed(1)+' KB');
      var action = item.is_dir ? `loadTree('${item.path.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}')` : `openFile('${item.path.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}')`;
      var ignoreAction = `ignorePath('${item.path.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}', ${item.is_dir})`;
      html += '<div class="tree-item" style="padding-right:8px;">'
        + '<div style="display:flex;align-items:center;gap:8px;flex:1;overflow:hidden;" onclick="' + action + '">'
        + '<div class="tree-icon">' + icon + '</div>'
        + '<div class="tree-name" title="' + esc(item.name) + '">' + esc(item.name) + '</div>'
        + '<div class="tree-size">' + sizeTxt + '</div>'
        + '</div>'
        + '<button class="tree-btn" style="padding:3px 6px;color:var(--err);border-color:var(--border);background:var(--surface);flex-shrink:0;" onclick="event.stopPropagation(); ' + ignoreAction + '" title="Ajouter aux ignorés">Ignorer</button>'
        + '</div>';
    }
    list.innerHTML = html;
  } catch(e) {
    list.innerHTML = '<div class="empty" style="color:var(--err)">' + esc(e.message) + '</div>';
  }
}
function openFile(path, isDeleted) {
  var url = '/api/open?path=' + encodeURIComponent(path) + (isDeleted ? '&dir_only=1' : '');
  fetch(url).then(r=>r.json()).then(d=>{
    if(!d.ok) console.warn("Impossible d'ouvrir ce fichier");
  });
}

async function openFiltersModal() {
  document.getElementById('filters-modal').classList.add('show');
  await loadFilters();
}
function closeFiltersModal() {
  document.getElementById('filters-modal').classList.remove('show');
}
async function loadFilters() {
  var tf = document.getElementById('filters-text');
  tf.value = "Chargement...";
  try {
    var r = await fetch('/api/filters');
    var d = await r.json();
    tf.value = d.content || (d.error ? "Erreur: " + d.error : "");
    tf.scrollTop = tf.scrollHeight;
  } catch(e) {
    tf.value = "Erreur de connexion";
  }
}
async function addFilter() {
  var input = document.getElementById('new-filter-input');
  var rule = input.value.trim();
  if(!rule) return;
  if (!rule.startsWith('- ') && !rule.startsWith('+ ') && !rule.startsWith('#')) {
    rule = '- ' + rule;
  }
  var tf = document.getElementById('filters-text');
  tf.value += (tf.value.endsWith('\n') ? '' : '\n') + rule + '\n';
  input.value = '';
  tf.scrollTop = tf.scrollHeight;
  await saveFilters();
}
async function saveFilters() {
  var tf = document.getElementById('filters-text');
  var btn = document.getElementById('save-filters-btn');
  var oldText = btn.textContent;
  btn.disabled = true;
  btn.textContent = 'Sauvegarde...';
  try {
    var r = await fetch('/api/filters_save', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({content: tf.value})
    });
    var d = await r.json();
    if(d.ok) {
      btn.textContent = '✓ Sauvegardé';
      setTimeout(() => { btn.textContent = oldText; }, 2000);
    } else {
      alert("Erreur: " + d.error);
      btn.textContent = oldText;
    }
  } catch(e) {
    alert("Erreur: " + e.message);
    btn.textContent = oldText;
  } finally {
    btn.disabled = false;
  }
}
async function ignorePath(path, isDir) {
  var rule = isDir ? "- " + path + "/**" : "- " + path;
  if(!confirm("Voulez-vous ajouter cette règle aux fichiers ignorés ?\n\n" + rule)) return;
  try {
    var r = await fetch('/api/filters_add?rule=' + encodeURIComponent(rule));
    var d = await r.json();
    if(d.ok) {
      alert("Ajouté avec succès !");
    } else {
      alert("Erreur : " + d.error);
    }
  } catch (e) {}
}

function openBwModal() { 
  document.getElementById('bw-modal').classList.add('show');
  fetch('/api/bwlimit').then(r=>r.json()).then(d=>{
    if(d.limit) document.getElementById('bw-select').value = d.limit;
  });
}
function closeBwModal() { document.getElementById('bw-modal').classList.remove('show'); }
async function saveBw() {
  var btn = document.getElementById('save-bw-btn');
  var limit = document.getElementById('bw-select').value;
  btn.disabled = true;
  btn.textContent = 'Enregistrement...';
  try {
    var r = await fetch('/api/bwlimit_save?limit=' + encodeURIComponent(limit));
    var d = await r.json();
    if(d.ok) {
      alert("Limite appliquée avec succès ! Elle sera active à la prochaine synchronisation.");
      closeBwModal();
    } else {
      alert("Erreur : " + d.error);
    }
  } catch(e) {
    alert("Erreur réseau");
  } finally {
    btn.disabled = false;
    btn.textContent = 'Appliquer la limite';
  }
}

function openDryRunModal() { document.getElementById('dryrun-modal').classList.add('show'); }
function closeDryRunModal() { document.getElementById('dryrun-modal').classList.remove('show'); }
async function startDryRun() {
  var btn = document.getElementById('start-dryrun-btn');
  var out = document.getElementById('dryrun-output');
  btn.disabled = true;
  btn.textContent = 'Simulation en cours... (peut prendre 1-2 minutes)';
  out.textContent = 'Lancement de rclone en mode --dry-run...\nAnalyse des changements en cours...';
  
  try {
    var r = await fetch('/api/dryrun');
    var d = await r.json();
    if(d.ok) {
      out.textContent = d.log || 'Aucun changement détecté (tout est à jour).';
    } else {
      out.textContent = 'Erreur : ' + d.error;
    }
  } catch(e) {
    out.textContent = 'Erreur réseau : ' + e.message;
  } finally {
    btn.disabled = false;
    btn.textContent = 'Relancer la simulation';
  }
}

/* ═══════════════════════════════════════════════════
   BOOT
   ═══════════════════════════════════════════════════ */
refresh();
_interval = setInterval(refresh, 10000);
