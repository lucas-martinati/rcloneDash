/* ═══════════════════════════════════════════════════
   STATE
   ═══════════════════════════════════════════════════ */
let _theme = document.documentElement.dataset.theme || 'dark';
let _llc = '';
let _interval = null;
let _curState = '';
let _logFilter = 'all';
let _lastLogs = [];
let _nextSyncTs = null;   // timestamp (ms) de la prochaine sync planifiée
let _lastStartTs = null;  // timestamp (ms) du dernier déclenchement
let _isSyncing = false;
let _livePct = null;      // % de transfert connu pendant une sync
let _runsSig = '';        // signatures des dernières données rendues,
let _recentSig = '';      // pour ne pas reconstruire le DOM inutilement

/* ═══════════════════════════════════════════════════
   UTILS
   ═══════════════════════════════════════════════════ */
function esc(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
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
  var d = new Date(ts);
  if (isNaN(d)) return ts.slice(0, 16);
  var now = new Date();
  var yest = new Date(now); yest.setDate(now.getDate() - 1);
  var hm = d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
  if (d.toDateString() === now.toDateString()) return "Auj. " + hm;
  if (d.toDateString() === yest.toDateString()) return 'Hier ' + hm;
  return d.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' }) + ' ' + hm;
}

function spin(v) { document.getElementById('spin').classList.toggle('on', v); }

function fmtSize(bytes) {
  if (bytes == null || isNaN(bytes)) return '';
  if (bytes < 1024) return bytes + ' o';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' Ko';
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' Mo';
  if (bytes < 1024 ** 4) return (bytes / (1024 ** 3)).toFixed(1) + ' Go';
  return (bytes / (1024 ** 4)).toFixed(2) + ' To';
}

/* "2m9.5s" / "1h2m3s" / "45.6s" → secondes */
function parseElapsed(e) {
  if (!e) return 0;
  var s = 0, m;
  if ((m = e.match(/(\d+)h/))) s += parseInt(m[1]) * 3600;
  if ((m = e.match(/(\d+)m(?!s)/))) s += parseInt(m[1]) * 60;
  if ((m = e.match(/([\d.]+)s/))) s += parseFloat(m[1]);
  return s;
}

function fmtRemaining(s) {
  if (s <= 0) return 'imminente…';
  if (s < 60) return Math.round(s) + ' s';
  if (s < 3600) return Math.floor(s / 60) + ' min ' + String(Math.round(s % 60)).padStart(2, '0') + ' s';
  return Math.floor(s / 3600) + ' h ' + String(Math.floor((s % 3600) / 60)).padStart(2, '0') + ' min';
}

/* ═══════════════════════════════════════════════════
   TOASTS
   ═══════════════════════════════════════════════════ */
function toast(msg, type) {
  var c = document.getElementById('toasts');
  var t = document.createElement('div');
  t.className = 'toast ' + (type || '');
  t.textContent = msg;
  c.appendChild(t);
  requestAnimationFrame(function () { t.classList.add('in'); });
  setTimeout(function () {
    t.classList.remove('in');
    setTimeout(function () { t.remove(); }, 300);
  }, 4200);
}

/* ═══════════════════════════════════════════════════
   THÈME
   ═══════════════════════════════════════════════════ */
function applyThemeIcon() {
  document.getElementById('ti').innerHTML = _theme === 'dark'
    ? '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>'
    : '<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>';
}

function toggleTheme() {
  _theme = _theme === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = _theme;
  localStorage.setItem('dash_theme', _theme);
  applyThemeIcon();
}

/* ═══════════════════════════════════════════════════
   BADGE D'ÉTAT
   ═══════════════════════════════════════════════════ */
function badge(state) {
  var b = document.getElementById('sbadge');
  var l = document.getElementById('slbl');
  var d = document.getElementById('sdot');
  var map = {
    running: ['run', 'Synchronisation…'],
    success: ['ok', 'À jour'],
    failed: ['err', 'Erreur'],
    idle: ['idle', 'En attente']
  };
  var info = map[state] || ['idle', state];
  b.className = 'badge ' + info[0];
  l.textContent = info[1];
  d.className = 'dot' + (state === 'running' ? ' pulse-anim' : '');

  var bsync = document.getElementById('bsync');
  var bcancel = document.getElementById('bcancel');
  bsync.style.display = state === 'running' ? 'none' : 'inline-flex';
  bcancel.style.display = state === 'running' ? 'inline-flex' : 'none';
}

/* ═══════════════════════════════════════════════════
   AUTO-REFRESH ADAPTATIF
   ═══════════════════════════════════════════════════ */
function setSmartRefresh(state) {
  if (state === _curState) return;
  _curState = state;
  if (_interval) clearInterval(_interval);
  var ms = state === 'running' ? 3000 : state === 'failed' ? 5000 : 10000;
  _interval = setInterval(refresh, ms);
}

/* ═══════════════════════════════════════════════════
   POULS — cycle de synchronisation
   ═══════════════════════════════════════════════════ */
function updatePulse(d) {
  var runs = d.runs || [];
  var last = null;
  for (var i = 0; i < runs.length; i++) {
    if (runs[i].status !== 'running') { last = runs[i]; break; }
  }

  // Dernière sync
  var dot = document.getElementById('pulse-dot');
  var lastEl = document.getElementById('pulse-last');
  _isSyncing = d.service.state === 'running' || (d.live && d.live.is_syncing);
  if (_isSyncing) {
    dot.className = 'pulse-dot run';
  } else if (last) {
    dot.className = 'pulse-dot ' + (last.status === 'success' ? 'ok' : 'err');
  } else {
    dot.className = 'pulse-dot';
  }
  if (last) {
    var mark = last.status === 'success'
      ? '<span class="st-ok">✓</span>' : '<span class="st-err">✗</span>';
    lastEl.innerHTML = esc(fmtT(last.start)) + ' ' + mark
      + (last.elapsed ? ' <span style="color:var(--muted)">· ' + esc(last.elapsed) + '</span>' : '');
  } else {
    lastEl.textContent = '—';
  }

  // Phase en cours (pendant une sync)
  var mid = document.getElementById('pulse-mid');
  if (_isSyncing && d.live && d.live.phase) {
    mid.style.display = '';
    mid.textContent = d.live.phase;
  } else {
    mid.style.display = 'none';
  }

  // Prochaine sync : on parse la date du trigger systemd
  var cap = document.getElementById('pulse-next-cap');
  var next = document.getElementById('pulse-next');
  var raw = (d.timer && d.timer.next_run) || '';
  var m = raw.match(/(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})/);
  if (d.timer && !d.timer.active) {
    _nextSyncTs = null;
    cap.textContent = 'Planification';
    next.textContent = 'Timer inactif';
    next.style.color = 'var(--warn)';
  } else if (!m && _isSyncing) {
    // Pendant une sync, systemd affiche "n/a" pour le prochain déclenchement
    _nextSyncTs = null;
    cap.textContent = 'Prochaine sync';
    next.textContent = 'après celle-ci';
    next.style.color = '';
  } else if (m) {
    _nextSyncTs = new Date(m[1] + 'T' + m[2]).getTime();
    cap.textContent = 'Prochaine sync';
    next.style.color = '';
  } else {
    _nextSyncTs = null;
    cap.textContent = 'Prochaine sync';
    next.textContent = raw && raw !== '—' ? raw : '—';
    next.style.color = '';
  }

  if (runs.length) _lastStartTs = new Date(runs[0].start).getTime() || null;

  _livePct = (_isSyncing && d.live && d.live.transfer && d.live.transfer.pct != null)
    ? d.live.transfer.pct : null;

  document.getElementById('pulse').classList.toggle('syncing', _isSyncing);
  tickPulse();
}

/* Tick 1 s : compte à rebours + ligne de vie */
function tickPulse() {
  var next = document.getElementById('pulse-next');
  var line = document.getElementById('pulse-line');

  if (_isSyncing) {
    line.style.transform = '';
    if (_livePct != null && _livePct > 0) {
      line.classList.remove('indet');
      line.style.width = _livePct + '%';
    } else {
      line.classList.add('indet');
    }
  } else {
    line.classList.remove('indet');
  }

  if (_nextSyncTs) {
    var remS = (_nextSyncTs - Date.now()) / 1000;
    next.textContent = 'dans ' + fmtRemaining(remS);
    if (!_isSyncing) {
      var cycleS = 600;
      if (_lastStartTs && _nextSyncTs > _lastStartTs) {
        cycleS = (_nextSyncTs - _lastStartTs) / 1000;
      }
      var pct = Math.min(100, Math.max(0, (1 - remS / cycleS) * 100));
      line.style.width = pct + '%';
    }
  } else if (!_isSyncing) {
    line.style.width = '0';
  }
}

/* ═══════════════════════════════════════════════════
   ALERTES & QUOTA
   ═══════════════════════════════════════════════════ */
function updateQuota(q) {
  var txt = document.getElementById('quota-text');
  var sub = document.getElementById('quota-sub');
  var bar = document.getElementById('quota-bar');
  if (!q) return;
  if (q.error) {
    txt.textContent = 'Erreur';
    txt.style.color = 'var(--err)';
    sub.textContent = 'quota Google Drive indisponible';
    sub.title = q.error;
    return;
  }
  var used = q.used || 0;
  var total = q.total || 1;
  var pct = Math.min(100, Math.round((used / total) * 100));
  txt.textContent = fmtSize(used);
  txt.style.color = '';
  sub.textContent = 'sur ' + fmtSize(total) + ' — ' + pct + ' %';
  sub.title = '';
  bar.style.width = Math.max(pct, 0.5) + '%';
  bar.classList.toggle('danger', pct > 90);
}

function updateAlerts(data) {
  updateQuota(data.quota);

  var ban = document.getElementById('alert-banner');
  var msg = document.getElementById('alert-msg');
  var kpis = data.kpis;
  var disk = data.disk;
  var live = data.live;

  if (kpis.consecutive_failures >= 2) {
    ban.className = 'alert-banner show-err';
    msg.textContent = kpis.consecutive_failures + ' syncs consécutives en erreur — '
      + (kpis.last_error_msg || 'consultez le journal pour le détail');
  } else if (live && live.is_syncing && live.duration_s > 300) {
    ban.className = 'alert-banner show-warn';
    msg.textContent = 'Synchronisation longue — en cours depuis '
      + Math.floor(live.duration_s / 60) + ' min ' + (live.duration_s % 60) + ' s';
  } else if (disk.pct > 90) {
    ban.className = 'alert-banner show-err';
    msg.textContent = 'Disque local rempli à ' + disk.pct + ' % — libérez de l\'espace';
  } else {
    ban.className = 'alert-banner';
  }

  var sb = document.getElementById('slow-badge');
  sb.style.display = (live && live.is_syncing && live.duration_s > 300) ? '' : 'none';
}

/* ═══════════════════════════════════════════════════
   KPIs
   ═══════════════════════════════════════════════════ */
function updateKPIs(data) {
  var disk = data.disk;
  var runs = data.runs || [], kpis = data.kpis;

  // Disque local
  var dkpi = document.getElementById('disk-kpi');
  var dkEl = document.getElementById('kdk');
  dkEl.textContent = disk.used + ' Go';
  document.getElementById('kdks').textContent = disk.free + ' Go libres sur ' + disk.total + ' Go';
  document.getElementById('dfill').style.width = disk.pct + '%';
  var diskDanger = disk.pct > 90;
  document.getElementById('dfill').classList.toggle('danger', diskDanger);
  dkpi.classList.toggle('danger', diskDanger);
  dkEl.style.color = diskDanger ? 'var(--err)' : '';

  // Syncs aujourd'hui
  var today = new Date().toISOString().slice(0, 10);
  var td = runs.filter(function (r) { return r.start && r.start.startsWith(today); });
  var tok = td.filter(function (r) { return r.status === 'success'; }).length;
  var terr = td.filter(function (r) { return r.status === 'failed'; }).length;
  document.getElementById('kr').textContent = td.length;
  document.getElementById('krs').textContent = tok + ' réussie(s)' + (terr ? ' · ' + terr + ' en erreur' : '');

  // Fichiers, vitesse, conflits
  document.getElementById('kf').textContent = kpis.total_files > 0
    ? kpis.total_files.toLocaleString('fr-FR') : '—';
  document.getElementById('ksp').textContent = kpis.avg_speed || '—';

  var kcEl = document.getElementById('kc');
  kcEl.textContent = kpis.conflicts_today;
  kcEl.style.color = kpis.conflicts_today > 0 ? 'var(--warn)' : '';
  document.getElementById('conflict-kpi').classList.toggle('danger', kpis.conflicts_today > 0);

  // Fiabilité 7 jours
  var rateVal = kpis.success_rate_7d;
  document.getElementById('ksr').textContent = rateVal + ' %';
  document.getElementById('ksr').style.color = rateVal < 90 ? 'var(--err)' : rateVal < 99 ? 'var(--warn)' : '';
  document.getElementById('srfill').style.width = rateVal + '%';
  document.getElementById('srfill').classList.toggle('danger', rateVal < 90);
}

/* ═══════════════════════════════════════════════════
   SYNC EN COURS
   ═══════════════════════════════════════════════════ */
function updateLive(live) {
  var section = document.getElementById('live-section');
  if (!live || (!live.is_syncing && live.phase_index <= 0)) {
    section.classList.remove('active');
    return;
  }
  section.classList.add('active');

  // Étapes
  var phaseNames = ['Listings', 'Diffs locaux', 'Diffs distants', 'Application', 'Mise à jour', 'Terminé'];
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

  // Durée écoulée
  var elapsed = document.getElementById('live-elapsed');
  if (live.duration_s > 0) {
    var mn = Math.floor(live.duration_s / 60);
    var s = live.duration_s % 60;
    elapsed.textContent = (mn > 0 ? mn + ' min ' : '') + s + ' s';
  } else {
    elapsed.textContent = (live.transfer && live.transfer.elapsed) || '';
  }

  // Stats de transfert
  var t = live.transfer || {};
  document.getElementById('tf-done').textContent = t.done || '—';
  document.getElementById('tf-total').textContent = t.total || '—';
  document.getElementById('tf-pct').textContent = t.pct != null ? t.pct + ' %' : '—';
  document.getElementById('tf-speed').textContent = t.speed || '—';
  document.getElementById('tf-eta').textContent = t.eta || '—';
  document.getElementById('tf-checks').textContent =
    t.checks_done != null ? t.checks_done + ' / ' + t.checks_total : '—';
  document.getElementById('tf-files').textContent =
    t.files_done != null ? t.files_done + ' / ' + t.files_total : '—';
  document.getElementById('tf-bar').style.width = (t.pct || 0) + '%';

  // Fichiers actifs
  var aw = document.getElementById('active-wrap');
  if (live.active_files && live.active_files.length > 0) {
    aw.style.display = '';
    var html = '<div class="kl">' + (live.active_files.length > 1
      ? 'Fichiers en cours de transfert (' + live.active_files.length + ')'
      : 'Fichier en cours de transfert') + '</div>';
    for (var i = 0; i < live.active_files.length; i++) {
      var f = live.active_files[i];
      var details = [];
      if (f.status === 'checking') details.push('vérification…');
      if (f.size) details.push(f.size);
      if (f.speed) details.push(f.speed);
      if (f.eta && f.eta !== '-') details.push('ETA ' + f.eta);
      html += '<div style="margin:6px 0 8px;">'
        + '<div style="display:flex; justify-content:space-between; align-items:baseline; gap:10px;">'
        + '<div class="active-fname" style="flex:1;" title="' + esc(f.name) + '">' + esc(f.name) + ' (' + f.pct + ' %)</div>'
        + '<div class="recent-size">' + esc(details.join(' · ')) + '</div>'
        + '</div>'
        + '<div class="tf-pbar"><div class="tf-pfill" style="width:' + f.pct + '%"></div></div>'
        + '</div>';
    }
    aw.innerHTML = html;
  } else if (live.active_file) {
    aw.style.display = '';
    aw.innerHTML = '<div class="kl">Fichier en cours de transfert</div>'
      + '<div class="active-fname">' + esc(live.active_file) + '</div>'
      + '<div class="tf-pbar"><div class="tf-pfill" style="width:' + live.active_file_pct + '%"></div></div>';
  } else {
    aw.style.display = 'none';
  }

  // Fichiers synchronisés durant la session
  var lsw = document.getElementById('live-synced-wrap');
  var lsl = document.getElementById('live-synced-list');
  if (live.synced_files && live.synced_files.length > 0) {
    lsw.style.display = '';
    var labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
    var html = '';
    for (var i = live.synced_files.length - 1; i >= 0; i--) {
      var f = live.synced_files[i];
      var cls = f.action || 'new';
      var pathArg = esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'");
      var actionCall = "openFile('" + pathArg + "'" + (cls === 'deleted' ? ', true' : '') + ')';
      var sizeTxt = fmtSize(f.size);
      html += '<div class="recent-item" style="padding:4px 0;" onclick="' + actionCall + '">'
        + '<span class="recent-dot ' + cls + '"></span>'
        + '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + '</span>'
        + '<span class="recent-path" title="' + esc(f.path) + '">' + esc(f.path) + '</span>'
        + (sizeTxt ? '<span class="recent-size">' + sizeTxt + '</span>' : '')
        + '<span class="recent-time">' + esc(f.time) + '</span>'
        + '</div>';
    }
    lsl.innerHTML = html;
  } else {
    lsw.style.display = 'none';
  }

  renderChanges('ch-p1', live.changes.path1);
  renderChanges('ch-p2', live.changes.path2);
}

function renderChanges(id, ch) {
  var el = document.getElementById(id);
  var items = [];
  var kinds = [['new', 'new'], ['modified', 'modified'], ['deleted', 'deleted']];
  for (var k = 0; k < kinds.length; k++) {
    var arr = ch[kinds[k][0]] || [];
    for (var i = 0; i < arr.length; i++) {
      items.push('<div class="change-item"><span class="change-dot ' + kinds[k][1] + '"></span>' + esc(arr[i]) + '</div>');
    }
  }
  el.innerHTML = items.length ? items.join('') : '<span class="change-empty">Aucun changement</span>';
}

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

/* ═══════════════════════════════════════════════════
   HISTORIQUE
   ═══════════════════════════════════════════════════ */
function updateRuns(runs) {
  var sig = JSON.stringify(runs || []);
  if (sig === _runsSig) return;
  _runsSig = sig;

  window._errorLogsCache = {};
  var tb = document.getElementById('rtb');
  var em = document.getElementById('rem');
  if (!runs || !runs.length) {
    tb.innerHTML = '';
    em.style.display = '';
    document.getElementById('sparkline-wrap').innerHTML = '';
    return;
  }
  em.style.display = 'none';

  // Conserver les détails ouverts entre deux rafraîchissements
  var openSet = {};
  tb.querySelectorAll('tr.open').forEach(function (tr) { openSet[tr.dataset.start] = true; });

  var html = '';
  for (var i = 0; i < runs.length; i++) {
    var r = runs[i];
    var statusCls = r.status === 'success' ? 'ok' : r.status === 'failed' ? 'err' : 'run';
    var statusTxt = r.status === 'success' ? '✓ Réussie' : r.status === 'failed' ? '✗ Erreur' : '⟳ En cours';
    var isOpen = openSet[r.start];

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

    var detHtml = '<div class="run-details-box">';
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
      var labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
      for (var j = 0; j < Math.min(r.synced_files.length, 50); j++) {
        var f = r.synced_files[j];
        var cls = f.action || 'new';
        var pathArg = esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'");
        var actionCall = "openFile('" + pathArg + "'" + (cls === 'deleted' ? ', true' : '') + ')';
        var fSize = fmtSize(f.size);
        detHtml += '<div class="run-details-file recent-item" onclick="event.stopPropagation(); ' + actionCall + '">'
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

function toggleRunDetails(i) {
  var det = document.getElementById('run-det-' + i);
  if (!det) return;
  var open = det.style.display === 'none';
  det.style.display = open ? 'table-row' : 'none';
  var row = det.previousElementSibling;
  if (row) row.classList.toggle('open', open);
}

function copyErrorLogs(idx, btn) {
  var text = window._errorLogsCache && window._errorLogsCache[idx];
  if (!text) return;
  navigator.clipboard.writeText(text).then(function () {
    btn.textContent = 'Copié ✓';
    setTimeout(function () { btn.textContent = 'Copier'; }, 2000);
  });
}

/* ═══════════════════════════════════════════════════
   JOURNAL
   ═══════════════════════════════════════════════════ */
function logPassesFilter(l) {
  if (_logFilter === 'all') return true;
  if (_logFilter === 'files') return l.l === 'ok';
  return l.l === 'error' || l.l === 'warn';
}

function renderLogs() {
  var w = document.getElementById('lwrap');
  var sc = document.getElementById('lscroll');
  var atBot = sc.scrollHeight - sc.scrollTop - sc.clientHeight < 50;
  var html = '';
  var shown = 0;
  for (var i = 0; i < _lastLogs.length; i++) {
    if (!logPassesFilter(_lastLogs[i])) continue;
    html += '<div class="ll ' + _lastLogs[i].l + '">' + colorizeLog(_lastLogs[i].t) + '</div>';
    shown++;
  }
  if (!shown) {
    html = '<div class="empty">' + (_logFilter === 'all'
      ? 'Le journal est vide pour le moment.'
      : 'Aucune ligne de ce type dans le journal récent.') + '</div>';
  }
  w.innerHTML = html;
  if (atBot) sc.scrollTop = sc.scrollHeight;
}

function updateLogs(logs) {
  _lastLogs = logs || [];
  var sig = _lastLogs.length + '|'
    + (_lastLogs.length ? _lastLogs[0].t + '|' + _lastLogs[_lastLogs.length - 1].t : '');
  if (sig !== _llc) {
    _llc = sig;
    renderLogs();
  }
}

function setLogFilter(f, btn) {
  _logFilter = f;
  document.querySelectorAll('.chiprow .chip').forEach(function (c) { c.classList.remove('on'); });
  btn.classList.add('on');
  renderLogs();
}

function logsBot() {
  var s = document.getElementById('lscroll');
  s.scrollTop = s.scrollHeight;
}

/* ═══════════════════════════════════════════════════
   FICHIERS RÉCENTS
   ═══════════════════════════════════════════════════ */
function updateRecentFiles(files) {
  var sig = JSON.stringify(files || []);
  if (sig === _recentSig) return;
  _recentSig = sig;

  var list = document.getElementById('recent-list');
  var countEl = document.getElementById('recent-count');

  if (!files || !files.length) {
    list.innerHTML = '<div class="empty">Aucun fichier synchronisé récemment.<br>Les fichiers copiés, modifiés ou supprimés apparaîtront ici.</div>';
    countEl.textContent = '';
    return;
  }

  countEl.textContent = '· ' + files.length;
  var labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
  var reversed = files.slice().reverse();
  var html = '';

  for (var i = 0; i < reversed.length; i++) {
    var f = reversed[i];
    var cls = f.action || 'new';
    var pathArg = esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'");
    var actionCall = "openFile('" + pathArg + "'" + (cls === 'deleted' ? ', true' : '') + ')';
    var sizeTxt = fmtSize(f.size);
    html += '<div class="recent-item" onclick="' + actionCall + '" title="Ouvrir dans le gestionnaire de fichiers">'
      + '<span class="recent-dot ' + cls + '"></span>'
      + '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + '</span>'
      + '<span class="recent-path" title="' + esc(f.path) + '">' + esc(f.path) + '</span>'
      + (sizeTxt ? '<span class="recent-size">' + sizeTxt + '</span>' : '')
      + '<span class="recent-time">' + fmtT(f.time) + '</span>'
      + '</div>';
  }
  list.innerHTML = html;
  filterRecent();
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
   MODULES DÉPLAÇABLES (drag & drop + redimensionnement)
   ═══════════════════════════════════════════════════ */
var dragSrcEl = null;

function handleDragStart(e) {
  dragSrcEl = this;
  e.dataTransfer.effectAllowed = 'move';
  e.dataTransfer.setData('text/html', this.innerHTML);
  this.classList.add('dragging');
}

function handleDragOver(e) {
  if (e.preventDefault) { e.preventDefault(); }
  e.dataTransfer.dropEffect = 'move';
  return false;
}

function handleDragEnter(e) { this.classList.add('over'); }
function handleDragLeave(e) { this.classList.remove('over'); }

function handleDrop(e) {
  if (e.stopPropagation) { e.stopPropagation(); }
  if (dragSrcEl !== this) {
    var srcOrder = window.getComputedStyle(dragSrcEl).order;
    var destOrder = window.getComputedStyle(this).order;

    if (srcOrder === destOrder) {
      var panels = document.querySelectorAll('.drag-panel');
      panels.forEach(function (p, i) { p.style.order = p.style.order || i; });
      srcOrder = dragSrcEl.style.order;
      destOrder = this.style.order;
    }

    dragSrcEl.style.order = destOrder;
    this.style.order = srcOrder;

    var orderData = {};
    document.querySelectorAll('.drag-panel').forEach(function (p) {
      p.style.height = '';
      orderData[p.id] = p.style.order;
    });
    localStorage.setItem('dash_panel_order', JSON.stringify(orderData));
    updateFullWidthPanel();
  }
  return false;
}

function handleDragEnd(e) {
  this.classList.remove('dragging');
  document.querySelectorAll('.drag-panel').forEach(function (p) { p.classList.remove('over'); });
}

function initDragAndDrop() {
  var panels = document.querySelectorAll('.drag-panel');
  var savedOrder = JSON.parse(localStorage.getItem('dash_panel_order') || '{}');

  panels.forEach(function (panel, idx) {
    panel.style.order = savedOrder[panel.id] || idx;

    var header = panel.querySelector('.ph');
    if (header) {
      header.addEventListener('mouseenter', function () { panel.setAttribute('draggable', 'true'); });
      header.addEventListener('mouseleave', function () { panel.removeAttribute('draggable'); });
    }

    panel.addEventListener('dragstart', handleDragStart, false);
    panel.addEventListener('dragenter', handleDragEnter, false);
    panel.addEventListener('dragover', handleDragOver, false);
    panel.addEventListener('dragleave', handleDragLeave, false);
    panel.addEventListener('drop', handleDrop, false);
    panel.addEventListener('dragend', handleDragEnd, false);
  });

  updateFullWidthPanel();
  initResizer();
}

function updateFullWidthPanel() {
  var panels = Array.from(document.querySelectorAll('.drag-panel'));
  panels.sort(function (a, b) { return parseInt(a.style.order || 0) - parseInt(b.style.order || 0); });
  panels.forEach(function (p, idx) {
    p.classList.toggle('full-width', idx === 2);
  });
}

function initResizer() {
  var container = document.getElementById('modules-container');
  var isResizingH = false;
  var isResizingV = false;
  var startX, startY, startCol1, startCol2, startRow1, startRow2;

  var savedCol = localStorage.getItem('dash_col_ratio');
  if (savedCol) {
    var parts = savedCol.split(':');
    container.style.setProperty('--col1', parts[0] + 'fr');
    container.style.setProperty('--col2', parts[1] + 'fr');
  }
  var savedRow = localStorage.getItem('dash_row_sizes');
  if (savedRow) {
    var rParts = savedRow.split(':');
    container.style.setProperty('--row1', rParts[0] + 'px');
    container.style.setProperty('--row2', rParts[1] + 'px');
  }

  var hoverRaf = 0;
  var lastMx = 0, lastMy = 0;

  function updateHoverCursor(clientX, clientY) {
    var panels = Array.from(document.querySelectorAll('.drag-panel'));
    panels.sort(function (a, b) { return parseInt(a.style.order || 0) - parseInt(b.style.order || 0); });
    if (panels.length < 3) return;
    var p1 = panels[0].getBoundingClientRect();
    var p2 = panels[1].getBoundingClientRect();
    var p3 = panels[2].getBoundingClientRect();

    var isH = (clientX > p1.right - 5 && clientX < p2.left + 5 && clientY > p1.top && clientY < p1.bottom);
    var isV = (clientY > p1.bottom - 5 && clientY < p3.top + 5 && clientX > p3.left && clientX < p3.right);

    var cursor = isH && isV ? 'move' : isH ? 'col-resize' : isV ? 'row-resize' : '';
    if (container.style.cursor !== cursor) container.style.cursor = cursor;
  }

  container.addEventListener('mousemove', function (e) {
    if (isResizingH) {
      var rect = container.getBoundingClientRect();
      var dx = e.clientX - startX;
      var c1 = startCol1 + (dx / rect.width) * (startCol1 + startCol2);
      var c2 = startCol2 - (dx / rect.width) * (startCol1 + startCol2);
      if (c1 > 0.1 && c2 > 0.1) {
        container.style.setProperty('--col1', c1 + 'fr');
        container.style.setProperty('--col2', c2 + 'fr');
        localStorage.setItem('dash_col_ratio', c1 + ':' + c2);
      }
      return;
    }
    if (isResizingV) {
      var dy = e.clientY - startY;
      var r1 = startRow1 + dy;
      var r2 = startRow2 - dy;
      if (r1 > 100 && r2 > 100) {
        container.style.setProperty('--row1', r1 + 'px');
        container.style.setProperty('--row2', r2 + 'px');
        localStorage.setItem('dash_row_sizes', r1 + ':' + r2);
      }
      return;
    }

    lastMx = e.clientX; lastMy = e.clientY;
    if (hoverRaf) return;
    hoverRaf = requestAnimationFrame(function () {
      hoverRaf = 0;
      updateHoverCursor(lastMx, lastMy);
    });
  });

  container.addEventListener('mousedown', function (e) {
    if (container.style.cursor === 'col-resize' || container.style.cursor === 'move') {
      isResizingH = true;
      startX = e.clientX;
      startCol1 = parseFloat(getComputedStyle(container).getPropertyValue('--col1')) || 1;
      startCol2 = parseFloat(getComputedStyle(container).getPropertyValue('--col2')) || 1;
      e.preventDefault();
    }
    if (container.style.cursor === 'row-resize' || container.style.cursor === 'move') {
      isResizingV = true;
      startY = e.clientY;
      startRow1 = parseFloat(getComputedStyle(container).getPropertyValue('--row1')) || 300;
      startRow2 = parseFloat(getComputedStyle(container).getPropertyValue('--row2')) || 260;
      e.preventDefault();
    }
    if (isResizingH || isResizingV) document.body.style.cursor = container.style.cursor;
  });

  window.addEventListener('mouseup', function () {
    isResizingH = false;
    isResizingV = false;
    document.body.style.cursor = '';
  });
}

/* ═══════════════════════════════════════════════════
   REFRESH PRINCIPAL
   ═══════════════════════════════════════════════════ */
async function refresh() {
  spin(true);
  try {
    var r = await fetch('/api/status');
    if (!r.ok) throw new Error(r.status);
    var d = await r.json();

    badge(d.service.state);
    setSmartRefresh(d.service.state);
    updatePulse(d);
    updateAlerts(d);
    updateKPIs(d);
    updateLive(d.live);
    updateRuns(d.runs);
    updateLogs(d.logs);
    updateRecentFiles(d.recent_files);
    document.getElementById('ts').textContent = 'MàJ ' + fmtT(d.ts);
  } catch (e) {
    document.getElementById('ts').textContent = '⚠ serveur injoignable';
  } finally {
    spin(false);
  }
}

/* ═══════════════════════════════════════════════════
   ACTIONS SYNC
   ═══════════════════════════════════════════════════ */
async function doSync() {
  var b = document.getElementById('bsync');
  var lbl = document.getElementById('bsync-lbl');
  b.disabled = true;
  lbl.textContent = 'Démarrage…';
  try {
    var r = await fetch('/api/trigger');
    var d = await r.json();
    if (d.ok) {
      toast('Synchronisation lancée', 'ok');
    } else {
      toast('Impossible de lancer la synchronisation : ' + (d.error || 'erreur inconnue'), 'err');
    }
  } catch {
    toast('Serveur injoignable — synchronisation non lancée', 'err');
  }
  setTimeout(function () {
    b.disabled = false;
    lbl.textContent = 'Synchroniser';
  }, 3000);
  setTimeout(refresh, 1500);
}

async function cancelSync() {
  var b = document.getElementById('bcancel');
  var lbl = document.getElementById('bcancel-lbl');
  b.disabled = true;
  lbl.textContent = 'Arrêt…';
  try {
    await fetch('/api/cancel');
    toast('Arrêt de la synchronisation demandé', 'warn');
  } catch (e) {
    toast('Serveur injoignable', 'err');
  }
  setTimeout(function () {
    b.disabled = false;
    lbl.textContent = 'Arrêter';
  }, 3000);
  setTimeout(refresh, 1000);
}

/* ═══════════════════════════════════════════════════
   NAVIGATEUR DE FICHIERS
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
  list.innerHTML = '<div class="empty">Chargement…</div>';
  try {
    var r = await fetch('/api/tree?dir=' + encodeURIComponent(dir));
    var d = await r.json();
    if (d.error) throw new Error(d.error);
    _currentTreeDir = d.current_dir;
    pathEl.textContent = '/' + (_currentTreeDir || '');
    upBtn.disabled = !_currentTreeDir;

    if (!d.items || d.items.length === 0) {
      list.innerHTML = '<div class="empty">Ce dossier est vide.</div>';
      return;
    }
    var html = '';
    for (var i = 0; i < d.items.length; i++) {
      var item = d.items[i];
      var icon = item.is_dir
        ? '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>'
        : '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>';
      var sizeTxt = item.is_dir ? '' : fmtSize(item.size);
      var pathArg = item.path.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
      var action = item.is_dir ? "loadTree('" + pathArg + "')" : "openFile('" + pathArg + "')";
      html += '<div class="tree-item">'
        + '<div class="tree-item-main" onclick="' + action + '">'
        + '<div class="tree-icon">' + icon + '</div>'
        + '<div class="tree-name" title="' + esc(item.name) + '">' + esc(item.name) + '</div>'
        + '<div class="tree-size">' + sizeTxt + '</div>'
        + '</div>'
        + '<button class="tree-btn ignore" onclick="event.stopPropagation(); ignorePath(this, \'' + pathArg + '\', ' + item.is_dir + ')"'
        + ' title="Exclure de la synchronisation">Exclure</button>'
        + '</div>';
    }
    list.innerHTML = html;
  } catch (e) {
    list.innerHTML = '<div class="empty" style="color:var(--err)">' + esc(e.message) + '</div>';
  }
}

function openFile(path, isDeleted) {
  var url = '/api/open?path=' + encodeURIComponent(path) + (isDeleted ? '&dir_only=1' : '');
  fetch(url).then(function (r) { return r.json(); }).then(function (d) {
    if (!d.ok) toast('Impossible d\'ouvrir ce fichier', 'warn');
  });
}

/* Exclusion en deux clics : le premier arme le bouton, le second confirme */
async function ignorePath(btn, path, isDir) {
  var rule = isDir ? '- ' + path + '/**' : '- ' + path;
  if (!btn.dataset.armed) {
    btn.dataset.armed = '1';
    btn.textContent = 'Confirmer ?';
    btn.classList.add('armed');
    setTimeout(function () {
      btn.dataset.armed = '';
      btn.textContent = 'Exclure';
      btn.classList.remove('armed');
    }, 3000);
    return;
  }
  btn.dataset.armed = '';
  btn.textContent = 'Exclure';
  btn.classList.remove('armed');
  try {
    var r = await fetch('/api/filters_add?rule=' + encodeURIComponent(rule));
    var d = await r.json();
    if (d.ok) {
      toast('Exclu de la synchronisation : ' + rule, 'ok');
    } else {
      toast('Impossible d\'ajouter la règle : ' + d.error, 'err');
    }
  } catch (e) {
    toast('Serveur injoignable', 'err');
  }
}

/* ═══════════════════════════════════════════════════
   EXCLUSIONS (filtres rclone)
   ═══════════════════════════════════════════════════ */
async function openFiltersModal() {
  document.getElementById('filters-modal').classList.add('show');
  await loadFilters();
}

function closeFiltersModal() { document.getElementById('filters-modal').classList.remove('show'); }

function colorizeLog(text) {
  var e = esc(text);
  e = e.replace(/^(\d{4}[-\/]\d{2}[-\/]\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*)/gm, '<span style="opacity:0.5">$1</span>');
  e = e.replace(/ ERROR /g, ' <strong style="color:var(--err)">ERROR </strong>');
  e = e.replace(/ NOTICE(:| )/g, ' <strong style="color:var(--warn)">NOTICE</strong>$1');
  e = e.replace(/ INFO(:| )/g, ' <strong style="color:var(--run)">INFO  </strong>$1');
  e = e.replace(/ DEBUG(:| )/g, ' <strong style="color:var(--faint)">DEBUG </strong>$1');
  e = e.replace(/(Deleted .*|File was deleted.*)/g, '<span style="color:var(--err)">$1</span>');
  e = e.replace(/(Copied .*|File is new.*)/g, '<span style="color:var(--ok)">$1</span>');
  e = e.replace(/(Updated .*|File was modified.*)/g, '<span style="color:var(--warn)">$1</span>');
  return e;
}

async function loadFilters() {
  var tf = document.getElementById('filters-text');
  tf.value = 'Chargement…';
  try {
    var r = await fetch('/api/filters');
    var d = await r.json();
    tf.value = d.content || (d.error ? 'Erreur : ' + d.error : '');
    tf.scrollTop = tf.scrollHeight;
  } catch (e) {
    tf.value = 'Serveur injoignable';
  }
}

async function addFilter() {
  var input = document.getElementById('new-filter-input');
  var rule = input.value.trim();
  if (!rule) return;
  if (!rule.startsWith('- ') && !rule.startsWith('+ ') && !rule.startsWith('#')) {
    rule = '- ' + rule;
  }
  var tf = document.getElementById('filters-text');
  tf.value += (tf.value.endsWith('\n') || !tf.value ? '' : '\n') + rule + '\n';
  input.value = '';
  tf.scrollTop = tf.scrollHeight;
  await saveFilters();
}

async function saveFilters() {
  var tf = document.getElementById('filters-text');
  var btn = document.getElementById('save-filters-btn');
  btn.disabled = true;
  btn.textContent = 'Enregistrement…';
  try {
    var r = await fetch('/api/filters_save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content: tf.value })
    });
    var d = await r.json();
    if (d.ok) {
      toast('Exclusions enregistrées', 'ok');
    } else {
      toast('Enregistrement impossible : ' + d.error, 'err');
    }
  } catch (e) {
    toast('Serveur injoignable — exclusions non enregistrées', 'err');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Enregistrer';
  }
}

/* ═══════════════════════════════════════════════════
   LIMITE DE DÉBIT
   ═══════════════════════════════════════════════════ */
function openBwModal() {
  document.getElementById('bw-modal').classList.add('show');
  fetch('/api/bwlimit').then(function (r) { return r.json(); }).then(function (d) {
    if (d.limit != null) document.getElementById('bw-select').value = d.limit;
  });
}

function closeBwModal() { document.getElementById('bw-modal').classList.remove('show'); }

async function saveBw() {
  var btn = document.getElementById('save-bw-btn');
  var limit = document.getElementById('bw-select').value;
  btn.disabled = true;
  btn.textContent = 'Application…';
  try {
    var r = await fetch('/api/bwlimit_save?limit=' + encodeURIComponent(limit));
    var d = await r.json();
    if (d.ok) {
      toast(limit
        ? 'Limite de débit appliquée — active dès la prochaine sync'
        : 'Limite de débit supprimée — vitesse maximale rétablie', 'ok');
      closeBwModal();
    } else {
      toast('Impossible d\'appliquer la limite : ' + d.error, 'err');
    }
  } catch (e) {
    toast('Serveur injoignable — limite non appliquée', 'err');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Appliquer la limite';
  }
}

/* ═══════════════════════════════════════════════════
   SIMULATION (dry run)
   ═══════════════════════════════════════════════════ */
function openDryRunModal() { document.getElementById('dryrun-modal').classList.add('show'); }
function closeDryRunModal() { document.getElementById('dryrun-modal').classList.remove('show'); }

async function startDryRun() {
  var btn = document.getElementById('start-dryrun-btn');
  var out = document.getElementById('dryrun-output');
  btn.disabled = true;
  btn.textContent = 'Analyse en cours…';
  out.textContent = 'Analyse des différences entre le dossier local et Google Drive…\nCela peut prendre une à deux minutes.';

  try {
    var r = await fetch('/api/dryrun');
    var d = await r.json();
    if (d.ok) {
      out.innerHTML = colorizeLog(d.log || 'Aucun changement à appliquer : tout est déjà synchronisé.');
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
   BOOT
   ═══════════════════════════════════════════════════ */
applyThemeIcon();
initDragAndDrop();
refresh();
_interval = setInterval(refresh, 10000);
setInterval(tickPulse, 1000);
