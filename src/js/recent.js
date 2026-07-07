import { S } from './state.js';
import { renderFileRow } from './utils.js';

/* ═══════════════════════════════════════════════════
   FICHIERS RÉCENTS
   ═══════════════════════════════════════════════════ */
export function updateRecentFiles(files) {
  var sig = JSON.stringify(files || []);
  if (sig === S.recentSig) return;
  S.recentSig = sig;

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
    html += renderFileRow(reversed[i]);
  }
  list.innerHTML = html;
  filterRecent();
}

export function filterRecent() {
  var q = document.getElementById('recent-search').value.toLowerCase();
  var items = document.querySelectorAll('#recent-list .recent-item');
  for (var i = 0; i < items.length; i++) {
    var path = items[i].querySelector('.recent-path').textContent.toLowerCase();
    items[i].style.display = path.indexOf(q) !== -1 ? '' : 'none';
  }
}
