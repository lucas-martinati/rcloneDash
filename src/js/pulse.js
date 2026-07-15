import { S } from './state.js';
import { esc, fmtT, fmtRemaining } from './utils.js';

/* ═══════════════════════════════════════════════════
   POULS — cycle de synchronisation
   ═══════════════════════════════════════════════════ */
export function updatePulse(d) {
  let runs = d.runs || [];
  let last = null;
  for (let i = 0; i < runs.length; i++) {
    if (runs[i].status !== 'running') {
      last = runs[i];
      break;
    }
  }

  // Dernière sync
  let dot = document.getElementById('pulse-dot');
  let lastEl = document.getElementById('pulse-last');
  S.isSyncing = d.service.state === 'running' || (d.live && d.live.is_syncing);
  if (S.isSyncing) {
    dot.className = 'pulse-dot run';
  } else if (last) {
    dot.className = 'pulse-dot ' + (last.status === 'success' ? 'ok' : 'err');
  } else {
    dot.className = 'pulse-dot';
  }
  if (last) {
    let mark =
      last.status === 'success' ? '<span class="st-ok">✓</span>' : '<span class="st-err">✗</span>';
    lastEl.innerHTML =
      esc(fmtT(last.start)) +
      ' ' +
      mark +
      (last.elapsed ? ' <span style="color:var(--muted)">· ' + esc(last.elapsed) + '</span>' : '');
  } else {
    lastEl.textContent = '—';
  }

  // Phase en cours (pendant une sync)
  let mid = document.getElementById('pulse-mid');
  if (S.isSyncing && d.live && d.live.phase) {
    mid.style.display = '';
    mid.textContent = d.live.phase;
  } else {
    mid.style.display = 'none';
  }

  // Prochaine sync : on parse la date du trigger systemd
  let cap = document.getElementById('pulse-next-cap');
  let next = document.getElementById('pulse-next');
  let raw = (d.timer && d.timer.next_run) || '';
  let m = raw.match(/(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})/);
  if (d.timer && !d.timer.active) {
    S.nextSyncTs = null;
    cap.textContent = 'Planification';
    next.textContent = 'Timer inactif';
    next.style.color = 'var(--warn)';
  } else if (!m && S.isSyncing) {
    // Pendant une sync, systemd affiche "n/a" pour le prochain déclenchement
    S.nextSyncTs = null;
    cap.textContent = 'Prochaine sync';
    next.textContent = 'après celle-ci';
    next.style.color = '';
  } else if (m) {
    S.nextSyncTs = new Date(m[1] + 'T' + m[2]).getTime();
    cap.textContent = 'Prochaine sync';
    next.style.color = '';
  } else {
    S.nextSyncTs = null;
    cap.textContent = 'Prochaine sync';
    next.textContent = raw && raw !== '—' ? raw : '—';
    next.style.color = '';
  }

  if (runs.length) S.lastStartTs = new Date(runs[0].start).getTime() || null;

  S.livePct =
    S.isSyncing && d.live && d.live.transfer && d.live.transfer.pct != null
      ? d.live.transfer.pct
      : null;

  document.getElementById('pulse').classList.toggle('syncing', S.isSyncing);
  tickPulse();
}

/* Tick 1 s : compte à rebours + ligne de vie */
export function tickPulse() {
  let next = document.getElementById('pulse-next');
  let line = document.getElementById('pulse-line');

  if (S.isSyncing) {
    line.style.transform = '';
    if (S.livePct != null && S.livePct > 0) {
      line.classList.remove('indet');
      line.style.width = S.livePct + '%';
    } else {
      line.classList.add('indet');
    }
  } else {
    line.classList.remove('indet');
  }

  if (S.nextSyncTs) {
    let remS = (S.nextSyncTs - Date.now()) / 1000;
    next.textContent = 'dans ' + fmtRemaining(remS);
    if (!S.isSyncing) {
      let cycleS = 600;
      if (S.lastStartTs && S.nextSyncTs > S.lastStartTs) {
        cycleS = (S.nextSyncTs - S.lastStartTs) / 1000;
      }
      let pct = Math.min(100, Math.max(0, (1 - remS / cycleS) * 100));
      line.style.width = pct + '%';
    }
  } else if (!S.isSyncing) {
    line.style.width = '0';
  }
}
