import { S } from './state.js';
import { refresh } from './refresh.js';

/* ═══════════════════════════════════════════════════
   BADGE D'ÉTAT
   ═══════════════════════════════════════════════════ */
export function badge(state) {
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
export function setSmartRefresh(state) {
  if (state === S.curState) return;
  S.curState = state;
  if (S.interval) clearInterval(S.interval);
  var ms = state === 'running' ? 3000 : state === 'failed' ? 5000 : 10000;
  S.interval = setInterval(refresh, ms);
}
