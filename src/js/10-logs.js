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
