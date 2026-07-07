import { esc, renderFileRow } from './utils.js';

/* ═══════════════════════════════════════════════════
   SYNC EN COURS
   ═══════════════════════════════════════════════════ */
export function updateLive(live) {
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
      html += renderFileRow(live.synced_files[i], 'padding:4px 0;');
    }
    lsl.innerHTML = html;
  } else {
    lsw.style.display = 'none';
  }

  renderChanges('ch-p1', live.changes.path1);
  renderChanges('ch-p2', live.changes.path2);
}

export function renderChanges(id, ch) {
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
