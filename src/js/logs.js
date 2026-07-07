import { S } from './state.js';
import { colorizeLog } from './utils.js';

/* ═══════════════════════════════════════════════════
   JOURNAL
   ═══════════════════════════════════════════════════ */
export function logPassesFilter(l) {
  if (S.logFilter === 'all') return true;
  if (S.logFilter === 'files') return l.l === 'ok';
  return l.l === 'error' || l.l === 'warn';
}

export function renderLogs() {
  var w = document.getElementById('lwrap');
  var sc = document.getElementById('lscroll');
  var atBot = sc.scrollHeight - sc.scrollTop - sc.clientHeight < 50;
  var html = '';
  var shown = 0;
  for (var i = 0; i < S.lastLogs.length; i++) {
    if (!logPassesFilter(S.lastLogs[i])) continue;
    html += '<div class="ll ' + S.lastLogs[i].l + '">' + colorizeLog(S.lastLogs[i].t) + '</div>';
    shown++;
  }
  if (!shown) {
    html = '<div class="empty">' + (S.logFilter === 'all'
      ? 'Le journal est vide pour le moment.'
      : 'Aucune ligne de ce type dans le journal récent.') + '</div>';
  }
  w.innerHTML = html;
  if (atBot) sc.scrollTop = sc.scrollHeight;
}

export function updateLogs(logs) {
  S.lastLogs = logs || [];
  var sig = S.lastLogs.length + '|'
    + (S.lastLogs.length ? S.lastLogs[0].t + '|' + S.lastLogs[S.lastLogs.length - 1].t : '');
  if (sig !== S.llc) {
    S.llc = sig;
    renderLogs();
  }
}

export function setLogFilter(f, btn) {
  S.logFilter = f;
  document.querySelectorAll('.chiprow .chip').forEach(function (c) { c.classList.remove('on'); });
  btn.classList.add('on');
  renderLogs();
}

export function logsBot() {
  var s = document.getElementById('lscroll');
  s.scrollTop = s.scrollHeight;
}
