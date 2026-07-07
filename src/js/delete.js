/* ═══════════════════════════════════════════════════
   SUPPRESSION LOCALE  (aperçu + diff Drive + confirmation)
   ═══════════════════════════════════════════════════ */
let _del = { path: '' };

function openDeleteModal(path) {
  _del = { path: path };
  document.getElementById('delete-modal').classList.add('show');
  document.getElementById('del-path').textContent = '/' + path;
  document.getElementById('del-summary').innerHTML = '<div class="del-loading">Analyse du contenu…</div>';
  document.getElementById('del-drive').innerHTML = '';
  document.getElementById('del-files').innerHTML = '';
  document.getElementById('del-confirm').disabled = true;
  fetch('/api/delete_preview?path=' + encodeURIComponent(path)).then(function (r) { return r.json(); }).then(function (d) {
    if (!d.ok) {
      document.getElementById('del-summary').innerHTML = '<div class="del-banner danger">' + ICO_WARN + '<div>Erreur : ' + esc(d.error || '') + '</div></div>';
      return;
    }
    _del.ignored = d.ignored;
    var banner = d.ignored
      ? '<div class="del-banner ok">' + ICO_CHECK + '<div><b>Exclu de la synchronisation.</b> La suppression locale ne sera pas propagée au Drive : tu libères seulement de l\'espace sur ce PC.</div></div>'
      : '<div class="del-banner danger">' + ICO_WARN + '<div><b>Cet élément n\'est pas exclu de la synchronisation.</b> Le supprimer localement l\'effacera aussi du Drive au prochain bisync. Exclus-le d\'abord pour conserver la copie cloud.</div></div>';
    var noun = d.count > 1 ? 'fichiers' : 'fichier';
    banner += '<div class="del-count"><span class="del-big">' + d.count + '</span> ' + noun
      + ' · <b>' + esc(fmtSize(d.size) || '0 o') + '</b> à supprimer localement' + (d.truncated ? ' (aperçu partiel)' : '') + '</div>';
    document.getElementById('del-summary').innerHTML = banner;

    var fl = '';
    for (var i = 0; i < d.files.length; i++) {
      var f = d.files[i];
      f.action = 'deleted';
      f.path = d.is_dir ? (d.path === '.' ? f.path : d.path + '/' + f.path) : d.path;
      fl += renderFileRow(f, '', { hideTime: true, hideAction: true });
    }
    if (d.truncated) fl += '<div class="del-more">… et d\'autres fichiers non listés</div>';
    document.getElementById('del-files').innerHTML = fl;

    document.getElementById('del-drive').innerHTML =
      '<button class="btn btn-g del-drive-btn" onclick="runDriveCheck()">' + ICO_CLOUD + ' Comparer avec le Drive (rclone check)</button>';
    document.getElementById('del-confirm').disabled = false;
  }).catch(function () {
    document.getElementById('del-summary').innerHTML = '<div class="del-banner danger">Serveur injoignable</div>';
  });
}

function runDriveCheck() {
  var el = document.getElementById('del-drive');
  el.innerHTML = '<div class="del-drive-load"><span class="fm-spin"></span> Comparaison avec le Drive en cours…</div>';
  fetch('/api/drive_check?path=' + encodeURIComponent(_del.path)).then(function (r) { return r.json(); }).then(function (d) {
    if (!d.ok) {
      el.innerHTML = '<div class="del-banner warn">' + ICO_WARN + '<div>Comparaison impossible : ' + esc(d.error || '') + '</div></div>';
      return;
    }
    if (!d.exists) {
      el.innerHTML = '<div class="del-banner danger">' + ICO_WARN + '<div><b>Absent du Drive.</b> Aucune copie cloud détectée : supprimer localement perdrait définitivement ces données.</div></div>';
      return;
    }
    var c = d.counts;
    if (d.fully_backed) {
      el.innerHTML = '<div class="del-banner ok">' + ICO_CHECK + '<div><b>Intégralement présent sur le Drive.</b> ' + c.identical + ' fichier(s) identiques'
        + (c.drive_only ? ' · ' + c.drive_only + ' en plus côté Drive' : '') + '. Suppression locale sans perte.</div></div>';
      return;
    }
    var lines = '';
    if (c.local_only) lines += '<div class="dc-line danger">' + c.local_only + ' fichier(s) uniquement en local — seraient perdus</div>';
    if (c.differ) lines += '<div class="dc-line warn">' + c.differ + ' fichier(s) différents de la version Drive</div>';
    if (c.error) lines += '<div class="dc-line warn">' + c.error + ' erreur(s) de lecture</div>';
    if (c.identical) lines += '<div class="dc-line ok">' + c.identical + ' fichier(s) identiques</div>';
    var det = '';
    (d.result.local_only || []).slice(0, 50).forEach(function (n) { det += '<div class="dc-item danger">+ ' + esc(n) + '</div>'; });
    (d.result.differ || []).slice(0, 50).forEach(function (n) { det += '<div class="dc-item warn">≠ ' + esc(n) + '</div>'; });
    el.innerHTML = '<div class="del-banner warn">' + ICO_WARN + '<div><b>Différences détectées.</b> Certains fichiers locaux ne sont pas (ou pas à jour) sur le Drive.</div></div>'
      + '<div class="dc-summary">' + lines + '</div>' + (det ? '<div class="dc-detail">' + det + '</div>' : '');
  }).catch(function () {
    el.innerHTML = '<div class="del-banner warn">Serveur injoignable</div>';
  });
}

function confirmDelete() {
  var cb = document.getElementById('del-confirm');
  cb.disabled = true;
  cb.textContent = 'Suppression…';
  fetch('/api/delete', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ path: _del.path }) })
    .then(function (r) { return r.json(); }).then(function (d) {
      cb.textContent = 'Supprimer localement';
      if (d.ok) {
        toast('Supprimé localement — ' + (fmtSize(d.freed) || '0 o') + ' libérés', 'ok');
        closeDeleteModal();
        if (document.getElementById('tree-modal').classList.contains('show')) loadTree(_fm.dir);
      } else {
        cb.disabled = false;
        toast('Échec de la suppression : ' + (d.error || ''), 'err');
      }
    }).catch(function () {
      cb.disabled = false;
      cb.textContent = 'Supprimer localement';
      toast('Serveur injoignable', 'err');
    });
}

function closeDeleteModal() {
  document.getElementById('delete-modal').classList.remove('show');
}
