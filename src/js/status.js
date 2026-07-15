import { S, bus } from './state.js';
import { refresh } from './refresh.js';

/* ═══════════════════════════════════════════════════
   BADGE D'ÉTAT
   ═══════════════════════════════════════════════════ */
export function badge(state) {
  let b = document.getElementById('sbadge');
  let l = document.getElementById('slbl');
  let d = document.getElementById('sdot');
  let map = {
    running: ['run', 'Synchronisation…'],
    success: ['ok', 'À jour'],
    failed: ['err', 'Erreur'],
    idle: ['idle', 'En attente']
  };
  let info = map[state] || ['idle', state];
  b.className = 'badge ' + info[0];
  l.textContent = info[1];
  d.className = 'dot' + (state === 'running' ? ' pulse-anim' : '');

  let bsync = document.getElementById('bsync');
  let bcancel = document.getElementById('bcancel');
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
  let ms = state === 'running' ? 3000 : state === 'failed' ? 5000 : 10000;
  S.interval = setInterval(refresh, ms);
}

bus.on('sync:status', (state) => {
  badge(state);
  setSmartRefresh(state);
});
