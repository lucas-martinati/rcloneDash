import { fmtSize } from './utils.js';

/* ═══════════════════════════════════════════════════
   ALERTES & QUOTA
   ═══════════════════════════════════════════════════ */
export function updateQuota(q) {
  let txt = document.getElementById('quota-text');
  let sub = document.getElementById('quota-sub');
  let bar = document.getElementById('quota-bar');
  if (!q) return;
  if (q.error) {
    txt.textContent = 'Erreur';
    txt.style.color = 'var(--err)';
    sub.textContent = 'quota Google Drive indisponible';
    sub.title = q.error;
    return;
  }
  let used = q.used || 0;
  let total = q.total || 1;
  let pct = Math.min(100, Math.round((used / total) * 100));
  txt.textContent = fmtSize(used);
  txt.style.color = '';
  sub.textContent = 'sur ' + fmtSize(total) + ' — ' + pct + ' %';
  sub.title = '';
  bar.style.width = Math.max(pct, 0.5) + '%';
  bar.classList.toggle('danger', pct > 90);
}

export function updateAlerts(data) {
  updateQuota(data.quota);

  let ban = document.getElementById('alert-banner');
  let msg = document.getElementById('alert-msg');
  let kpis = data.kpis;
  let disk = data.disk;
  let live = data.live;

  if (kpis.consecutive_failures >= 2) {
    ban.className = 'alert-banner show-err';
    msg.textContent =
      kpis.consecutive_failures +
      ' syncs consécutives en erreur — ' +
      (kpis.last_error_msg || 'consultez le journal pour le détail');
  } else if (live && live.is_syncing && live.duration_s > 300) {
    ban.className = 'alert-banner show-warn';
    msg.textContent =
      'Synchronisation longue — en cours depuis ' +
      Math.floor(live.duration_s / 60) +
      ' min ' +
      (live.duration_s % 60) +
      ' s';
  } else if (disk.pct > 90) {
    ban.className = 'alert-banner show-err';
    msg.textContent = 'Disque local rempli à ' + disk.pct + " % — libérez de l'espace";
  } else {
    ban.className = 'alert-banner';
  }

  let sb = document.getElementById('slow-badge');
  sb.style.display = live && live.is_syncing && live.duration_s > 300 ? '' : 'none';
}

/* ═══════════════════════════════════════════════════
   KPIs
   ═══════════════════════════════════════════════════ */
export function updateKPIs(data) {
  let disk = data.disk;
  let runs = data.runs || [],
    kpis = data.kpis;

  // Disque local
  let dkpi = document.getElementById('disk-kpi');
  let dkEl = document.getElementById('kdk');
  dkEl.textContent = disk.used + ' Go';
  document.getElementById('kdks').textContent = disk.free + ' Go libres sur ' + disk.total + ' Go';
  document.getElementById('dfill').style.width = disk.pct + '%';
  let diskDanger = disk.pct > 90;
  document.getElementById('dfill').classList.toggle('danger', diskDanger);
  dkpi.classList.toggle('danger', diskDanger);
  dkEl.style.color = diskDanger ? 'var(--err)' : '';

  // Syncs aujourd'hui
  let today = new Date().toISOString().slice(0, 10);
  let td = runs.filter(function (r) {
    return r.start && r.start.startsWith(today);
  });
  let tok = td.filter(function (r) {
    return r.status === 'success';
  }).length;
  let terr = td.filter(function (r) {
    return r.status === 'failed';
  }).length;
  document.getElementById('kr').textContent = td.length;
  document.getElementById('krs').textContent =
    tok + ' réussie(s)' + (terr ? ' · ' + terr + ' en erreur' : '');

  // Fichiers, vitesse, conflits
  document.getElementById('kf').textContent =
    kpis.total_files > 0 ? kpis.total_files.toLocaleString('fr-FR') : '—';
  document.getElementById('ksp').textContent = kpis.avg_speed || '—';

  let kcEl = document.getElementById('kc');
  kcEl.textContent = kpis.conflicts_today;
  kcEl.style.color = kpis.conflicts_today > 0 ? 'var(--warn)' : '';
  document.getElementById('conflict-kpi').classList.toggle('danger', kpis.conflicts_today > 0);

  // Fiabilité 7 jours
  let rateVal = kpis.success_rate_7d;
  document.getElementById('ksr').textContent = rateVal + ' %';
  document.getElementById('ksr').style.color =
    rateVal < 90 ? 'var(--err)' : rateVal < 99 ? 'var(--warn)' : '';
  document.getElementById('srfill').style.width = rateVal + '%';
  document.getElementById('srfill').classList.toggle('danger', rateVal < 90);
}
