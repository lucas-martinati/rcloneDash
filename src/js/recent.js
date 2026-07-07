import { S } from './state.js';
import { renderFileRow } from './utils.js';

/* ═══════════════════════════════════════════════════
   FICHIERS RÉCENTS
   ═══════════════════════════════════════════════════ */
export function updateRecentFiles(files) {
  let sig = JSON.stringify(files || []);
  if (sig === S.recentSig) return;
  S.recentSig = sig;

  let list = document.getElementById('recent-list');
  let countEl = document.getElementById('recent-count');

  if (!files || !files.length) {
    list.innerHTML = '<div class="empty">Aucun fichier synchronisé récemment.<br>Les fichiers copiés, modifiés ou supprimés apparaîtront ici.</div>';
    countEl.textContent = '';
    return;
  }

  countEl.textContent = '· ' + files.length;
  let labels = { 'new': 'Copié', modified: 'Modifié', deleted: 'Supprimé' };
  let reversed = files.slice().reverse();
  let html = '';

  for (let i = 0; i < reversed.length; i++) {
    html += renderFileRow(reversed[i]);
  }
  list.innerHTML = html;
  filterRecent();
}

export function filterRecent() {
  let q = document.getElementById('recent-search').value.toLowerCase();
  let items = document.querySelectorAll('#recent-list .recent-item');
  for (let i = 0; i < items.length; i++) {
    let path = items[i].querySelector('.recent-path').textContent.toLowerCase();
    items[i].style.display = path.indexOf(q) !== -1 ? '' : 'none';
  }
}
