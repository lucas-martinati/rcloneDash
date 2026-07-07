/* ═══════════════════════════════════════════════════
   NAVIGATEUR DE FICHIERS
   Fil d'ariane · recherche instantanée · tri par colonne ·
   icônes typées · navigation clavier · résumé de dossier
   ═══════════════════════════════════════════════════ */
let _fm = {
  dir: '',        // dossier courant (relatif à la racine synchronisée)
  items: [],      // items bruts renvoyés par l'API
  view: [],       // items filtrés + triés actuellement affichés
  sortKey: 'name',
  sortAsc: true,
  filter: '',
  sel: -1,        // index sélectionné au clavier dans `view`
  recursive: false,   // recherche étendue aux sous-dossiers
  searchResults: [],  // résultats renvoyés par /api/search
  searching: false,
  searchTimer: null,
  truncated: false
};

/* ── Icônes SVG (feather-style, viewBox 24) ── */

/* ── Ouverture / navigation ── */
function openTreeModal() {
  document.getElementById('tree-modal').classList.add('show');
  loadTree('');
  setTimeout(function () { var s = document.getElementById('fm-search'); if (s) s.focus(); }, 80);
}
function closeTreeModal() {
  document.getElementById('tree-modal').classList.remove('show');
}
function treeUp() {
  if (!_fm.dir) return;
  var parts = _fm.dir.split('/');
  parts.pop();
  loadTree(parts.join('/'));
}
function openCurrentDir() {
  fetch('/api/open?path=' + encodeURIComponent(_fm.dir)).then(function (r) { return r.json(); })
    .then(function (d) { if (!d.ok) toast('Impossible d\'ouvrir ce dossier', 'warn'); });
}

async function loadTree(dir) {
  var list = document.getElementById('fm-list');
  list.innerHTML = fmSkeleton();
  var s = document.getElementById('fm-search');
  s.value = '';
  document.getElementById('fm-search-clear').hidden = true;
  _fm.filter = '';
  _fm.sel = -1;
  _fm.searchResults = [];
  _fm.searching = false;
  clearTimeout(_fm.searchTimer);
  try {
    var r = await fetch('/api/tree?dir=' + encodeURIComponent(dir));
    var d = await r.json();
    if (d.error) throw new Error(d.error);
    _fm.dir = d.current_dir || '';
    _fm.items = d.items || [];
    document.getElementById('tree-up-btn').disabled = !_fm.dir;
    fmRenderCrumbs();
    fmRender();
  } catch (e) {
    list.innerHTML = '<div class="fm-empty err">' + _svg(FM_ICONS.file, 22) + '<span>' + esc(e.message) + '</span></div>';
    document.getElementById('fm-footer').textContent = '';
  }
}

/* ── Fil d'ariane cliquable ── */
function fmRenderCrumbs() {
  var c = document.getElementById('fm-crumbs');
  var html = '<button class="fm-crumb root" onclick="loadTree(\'\')" title="Racine synchronisée">'
    + _svg('<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>', 14)
    + '</button>';
  if (_fm.dir) {
    var parts = _fm.dir.split('/');
    var acc = '';
    for (var i = 0; i < parts.length; i++) {
      acc = acc ? acc + '/' + parts[i] : parts[i];
      var last = i === parts.length - 1;
      var pathArg = acc.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
      html += '<span class="fm-crumb-sep">›</span>'
        + '<button class="fm-crumb' + (last ? ' current' : '') + '" onclick="loadTree(\'' + pathArg + '\')">'
        + esc(parts[i]) + '</button>';
    }
  }
  c.innerHTML = html;
  c.scrollLeft = c.scrollWidth;
}

/* ── Tri / filtre ── */
function fmSort(key) {
  if (_fm.sortKey === key) _fm.sortAsc = !_fm.sortAsc;
  else { _fm.sortKey = key; _fm.sortAsc = key === 'name'; }   // nom ↑ ; taille/date ↓ par défaut
  fmRender();
}
function fmFilter(v) {
  _fm.filter = v.trim().toLowerCase();
  document.getElementById('fm-search-clear').hidden = !v;
  _fm.sel = -1;
  if (_fm.recursive) {
    // recherche récursive : appel serveur débounce
    clearTimeout(_fm.searchTimer);
    if (_fm.filter.length < 2) {
      _fm.searchResults = [];
      _fm.searching = false;
      fmRender();
      return;
    }
    _fm.searching = true;
    fmRender();
    _fm.searchTimer = setTimeout(fmRunSearch, 250);
  } else {
    fmRender();
  }
}
function fmClearSearch() {
  var s = document.getElementById('fm-search');
  s.value = ''; s.focus();
  fmFilter('');
}
function fmToggleRecursive() {
  _fm.recursive = !_fm.recursive;
  var btn = document.getElementById('fm-scope');
  btn.classList.toggle('active', _fm.recursive);
  btn.setAttribute('aria-pressed', _fm.recursive ? 'true' : 'false');
  _fm.searchResults = [];
  _fm.sel = -1;
  // relance la requête courante dans le nouveau périmètre
  fmFilter(document.getElementById('fm-search').value);
}
async function fmRunSearch() {
  var q = _fm.filter;
  try {
    var r = await fetch('/api/search?dir=' + encodeURIComponent(_fm.dir) + '&q=' + encodeURIComponent(q));
    var d = await r.json();
    // ignore les réponses obsolètes (l'utilisateur a continué à taper)
    if (_fm.filter !== q || !_fm.recursive) return;
    _fm.searching = false;
    _fm.searchResults = d.items || [];
    _fm.truncated = !!d.truncated;
    fmRender();
  } catch (e) {
    _fm.searching = false;
    _fm.searchResults = [];
    fmRender();
  }
}

/* ── Rendu de la liste ── */
function fmRender() {
  var list = document.getElementById('fm-list');
  var recursiveActive = _fm.recursive && _fm.filter.length >= 2;

  // état « recherche en cours » (mode récursif)
  if (recursiveActive && _fm.searching) {
    list.innerHTML = '<div class="fm-empty"><span class="fm-spin"></span><span>Recherche dans les sous-dossiers…</span></div>';
    _fm.view = [];
    fmFooter();
    return;
  }

  var view;
  if (recursiveActive) {
    view = _fm.searchResults;
  } else {
    view = _fm.items;
    if (_fm.filter) view = view.filter(function (it) { return it.name.toLowerCase().indexOf(_fm.filter) !== -1; });
  }

  var key = _fm.sortKey, dir = _fm.sortAsc ? 1 : -1;
  view = view.slice().sort(function (a, b) {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;   // dossiers toujours en tête
    var av, bv;
    if (key === 'size') { av = a.is_dir ? -1 : (a.size || 0); bv = b.is_dir ? -1 : (b.size || 0); }
    else if (key === 'mtime') { av = a.mtime || 0; bv = b.mtime || 0; }
    else { av = a.name.toLowerCase(); bv = b.name.toLowerCase(); }
    if (av < bv) return -dir;
    if (av > bv) return dir;
    return a.name.toLowerCase() < b.name.toLowerCase() ? -1 : 1;
  });
  _fm.view = view;

  document.querySelectorAll('.fm-col-btn').forEach(function (b) {
    var on = b.dataset.key === _fm.sortKey;
    b.classList.toggle('active', on);
    b.classList.toggle('asc', on && _fm.sortAsc);
    b.classList.toggle('desc', on && !_fm.sortAsc);
  });

  if (!view.length) {
    list.innerHTML = _fm.filter
      ? '<div class="fm-empty">' + _svg('<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>', 22)
        + '<span>Aucun résultat pour « ' + esc(_fm.filter) + ' »</span></div>'
      : '<div class="fm-empty">' + _svg(FM_ICONS.folder, 22) + '<span>Ce dossier est vide</span></div>';
    fmFooter();
    return;
  }

  var html = '';
  for (var i = 0; i < view.length; i++) html += fmRow(view[i], i);
  list.innerHTML = html;
  fmFooter();
}

function fmRow(item, i) {
  var cat = fmCategory(item);
  var isDir = item.is_dir;
  var pathArg = item.path.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
  var action = isDir ? "loadTree('" + pathArg + "')" : "openFile('" + pathArg + "', false, event)";
  var meta = isDir
    ? (item.count == null ? '—' : item.count + (item.count > 1 ? ' éléments' : ' élément'))
    : (fmtSize(item.size) || '—');
  var modTxt = item.mtime ? fmtDT(item.mtime) : '—';
  var title = isDir ? 'Ouvrir le dossier' : 'Ouvrir le fichier — Ctrl+clic pour ouvrir son dossier';
  var ignored = !!item.ignored;
  var revealBtn = isDir
    ? '<button class="fm-act" onclick="event.stopPropagation(); openFile(\'' + pathArg + '\')" title="Ouvrir dans l\'explorateur système">'
      + _svg('<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>', 13) + '</button>'
    : '<button class="fm-act" onclick="event.stopPropagation(); openFile(\'' + pathArg + '\', true)" title="Ouvrir le dossier contenant">'
      + _svg(FM_ICONS.folder, 13) + '</button>';
  // Basculer exclusion : croix (exclure) si suivi, œil (ré-inclure) si déjà exclu
  var toggleBtn = ignored
    ? '<button class="fm-act reinc" onclick="event.stopPropagation(); reincludePath(\'' + pathArg + '\', ' + isDir + ')" title="Ré-inclure dans la synchronisation">'
      + _svg('<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>', 13) + '</button>'
    : '<button class="fm-act" onclick="event.stopPropagation(); ignorePath(\'' + pathArg + '\', ' + isDir + ')" title="Exclure de la synchronisation">'
      + _svg('<circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/>', 13) + '</button>';
  var delBtn = '<button class="fm-act danger" onclick="event.stopPropagation(); openDeleteModal(\'' + pathArg + '\')" title="Supprimer localement…">'
    + _svg('<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>', 13) + '</button>';
  return '<div class="fm-row' + (i === _fm.sel ? ' sel' : '') + (isDir ? ' is-dir' : ' file-link') + (ignored ? ' ignored' : '')
      + '" data-i="' + i + '" onclick="' + action + '" title="' + title + '">'
    + '<span class="fm-ic cat-' + cat + '">' + _svg(FM_ICONS[cat], 16) + '</span>'
    + '<span class="fm-name"><span class="fm-nm-txt">' + fmHighlight(item.name) + '</span>'
    + (ignored ? '<span class="fm-ig-badge" title="Exclu de la synchronisation">exclu</span>' : '') + '</span>'
    + '<span class="fm-meta col-mod" title="' + esc(modTxt) + '">' + esc(modTxt) + '</span>'
    + '<span class="fm-meta col-size">' + esc(meta) + '</span>'
    + '<span class="fm-row-actions">'
    + revealBtn + toggleBtn + delBtn
    + '</span>'
    + '</div>';
}

function fmHighlight(name) {
  if (!_fm.filter) return esc(name);
  var idx = name.toLowerCase().indexOf(_fm.filter);
  if (idx < 0) return esc(name);
  return esc(name.slice(0, idx)) + '<mark>' + esc(name.slice(idx, idx + _fm.filter.length))
    + '</mark>' + esc(name.slice(idx + _fm.filter.length));
}

function fmFooter() {
  var f = document.getElementById('fm-footer');
  // Mode recherche récursive : résumé des résultats
  if (_fm.recursive && _fm.filter.length >= 2) {
    var n = _fm.view.length;
    var left = 'Recherche dans les sous-dossiers';
    var right = (n ? n : 'Aucun') + ' résultat' + (n > 1 ? 's' : '') + (_fm.truncated ? ' (limité)' : '');
    f.innerHTML = '<span>' + esc(left) + '</span><span class="fm-foot-r">' + esc(right) + '</span>';
    return;
  }
  var dirs = 0, files = 0, total = 0;
  _fm.items.forEach(function (it) {
    if (it.is_dir) dirs++; else { files++; total += it.size || 0; }
  });
  var parts = [];
  if (dirs) parts.push(dirs + (dirs > 1 ? ' dossiers' : ' dossier'));
  if (files) parts.push(files + (files > 1 ? ' fichiers' : ' fichier'));
  var left2 = parts.join('  ·  ') || 'Dossier vide';
  if (total) left2 += '  ·  ' + fmtSize(total);
  var right2 = _fm.filter ? (_fm.view.length + ' résultat' + (_fm.view.length > 1 ? 's' : '')) : '';
  f.innerHTML = '<span>' + esc(left2) + '</span>' + (right2 ? '<span class="fm-foot-r">' + esc(right2) + '</span>' : '');
}

function fmSkeleton() {
  var row = '<div class="fm-skel"><span class="sk-ic"></span><span class="sk-l"></span><span class="sk-s"></span></div>';
  return row.repeat(8);
}

/* ── Navigation clavier (active seulement quand le modal est ouvert) ── */
function fmKeydown(e) {
  if (!document.getElementById('tree-modal').classList.contains('show')) return;
  // ne pas piloter la liste quand une modale est au premier plan
  if (document.getElementById('delete-modal').classList.contains('show')) return;
  if (document.getElementById('exclude-modal').classList.contains('show')) return;
  if (document.getElementById('reinc-modal').classList.contains('show')) return;
  var searchEl = document.getElementById('fm-search');
  if (e.key === '/' && document.activeElement !== searchEl) {
    e.preventDefault(); searchEl.focus(); return;
  }
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault();
    if (!_fm.view.length) return;
    if (_fm.sel < 0) _fm.sel = e.key === 'ArrowDown' ? 0 : _fm.view.length - 1;
    else _fm.sel = Math.max(0, Math.min(_fm.view.length - 1, _fm.sel + (e.key === 'ArrowDown' ? 1 : -1)));
    fmUpdateSel();
    return;
  }
  if (e.key === 'Enter') {
    var idx = _fm.sel >= 0 ? _fm.sel : (_fm.view.length ? 0 : -1);
    if (idx >= 0) {
      var it = _fm.view[idx];
      if (it.is_dir) loadTree(it.path);
      else openFile(it.path, false, e);   // Ctrl+Entrée → dossier contenant
    }
    return;
  }
  if (e.key === 'Backspace' && (document.activeElement !== searchEl || searchEl.value === '')) {
    e.preventDefault(); treeUp();
  }
}
function fmUpdateSel() {
  document.querySelectorAll('#fm-list .fm-row').forEach(function (r) {
    var on = +r.dataset.i === _fm.sel;
    r.classList.toggle('sel', on);
    if (on) r.scrollIntoView({ block: 'nearest' });
  });
}
window.addEventListener('keydown', fmKeydown);

function openFile(path, isDeleted, ev) {
  // Ctrl/⌘ maintenu : ouvrir le dossier parent plutôt que le fichier lui-même.
  // (les fichiers supprimés n'existent plus, on ouvre toujours leur dossier)
  var dirOnly = isDeleted || (ev && (ev.ctrlKey || ev.metaKey));
  var url = '/api/open?path=' + encodeURIComponent(path) + (dirOnly ? '&dir_only=1' : '');
  fetch(url).then(function (r) { return r.json(); }).then(function (d) {
    if (!d.ok) toast(dirOnly ? 'Impossible d\'ouvrir ce dossier' : 'Impossible d\'ouvrir ce fichier', 'warn');
  });
}
