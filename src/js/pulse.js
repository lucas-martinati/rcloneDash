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
