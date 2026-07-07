import { esc, fmtSize, renderFileRow } from './utils.js';
import { toast } from './toasts.js';
import { loadTree, _fm } from './file-browser.js';
import { ICO_WARN } from './icons.js';

/* ═══════════════════════════════════════════════════
   EXCLUSION  (impact + choix : exclure seul / +local / +drive / +les deux)
   ═══════════════════════════════════════════════════ */
export let _exc = { path: '', isDir: false };

export function ignorePath(path, isDir) { openExcludeModal(path, isDir); }

export function openExcludeModal(path, isDir) {
  _exc = { path: path, isDir: !!isDir };
  document.getElementById('exclude-modal').classList.add('show');
  document.getElementById('exc-path').textContent = '/' + path;
  // réinitialise le choix sur « exclure seulement »
  let noneRadio = document.querySelector('input[name="exc-action"][value="none"]');
  if (noneRadio) noneRadio.checked = true;
  excOnChoice();
  document.getElementById('exc-summary').innerHTML = '<div class="del-loading">Analyse du contenu…</div>';
  document.getElementById('exc-files').innerHTML = '';
  fetch('/api/delete_preview?path=' + encodeURIComponent(path)).then(function (r) { return r.json(); }).then(function (d) {
    if (!d.ok) {
      document.getElementById('exc-summary').innerHTML =
        '<div class="del-count">La règle <span class="mono">' + esc(excRule()) + '</span> sera ajoutée.</div>';
      return;
    }
    let noun = d.count > 1 ? 'fichiers' : 'fichier';
    document.getElementById('exc-summary').innerHTML =
      '<div class="del-count">Concerne <span class="del-big2">' + d.count + '</span> ' + noun
      + ' · <b>' + esc(fmtSize(d.size) || '0 o') + '</b> en local' + (d.truncated ? ' (aperçu partiel)' : '') + '</div>'
      + '<div class="exc-rule">Règle ajoutée : <span class="mono">' + esc(excRule()) + '</span></div>';
    let fl = '';
    d.files.forEach(function (f) {
      f.action = 'excluded';
      f.path = d.is_dir ? (d.path === '.' ? f.path : d.path + '/' + f.path) : d.path;
      fl += renderFileRow(f, '', { hideTime: true, hideAction: true });
    });
    if (d.truncated) fl += '<div class="del-more">… et d\'autres fichiers non listés</div>';
    document.getElementById('exc-files').innerHTML = fl;
  }).catch(function () {
    document.getElementById('exc-summary').innerHTML = '<div class="del-count">Serveur injoignable pour l\'aperçu.</div>';
  });
}

export function excRule() {
  return _exc.isDir ? '- ' + _exc.path + '/**' : '- ' + _exc.path;
}
export function excChoice() {
  let el = document.querySelector('input[name="exc-action"]:checked');
  return el ? el.value : 'none';
}
export function excOnChoice() {
  let v = excChoice();
  let btn = document.getElementById('exc-confirm');
  let labels = {
    none: 'Exclure',
    local: 'Exclure et supprimer en local',
    drive: 'Exclure et supprimer du Drive',
    both: 'Exclure et supprimer des deux'
  };
  btn.textContent = labels[v] || 'Exclure';
  btn.className = 'btn ' + (v === 'none' ? 'btn-g' : 'btn-danger');
  btn.disabled = false;
}

export async function confirmExclude() {
  let choice = excChoice();
  let btn = document.getElementById('exc-confirm');
  btn.disabled = true;
  let origLabel = btn.textContent;
  btn.textContent = 'En cours…';
  try {
    // 1) Toujours ajouter la règle d'exclusion en premier (empêche toute propagation bisync)
    let r = await fetch('/api/filters_add?rule=' + encodeURIComponent(excRule()));
    let d = await r.json();
    if (!d.ok) { toast('Impossible d\'exclure : ' + (d.error || ''), 'err'); btn.disabled = false; btn.textContent = origLabel; return; }

    let msgs = ['Exclu de la synchronisation'];
    // 2) Suppression locale si demandée
    if (choice === 'local' || choice === 'both') {
      let rl = await fetch('/api/delete', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ path: _exc.path }) });
      let dl = await rl.json();
      if (dl.ok) msgs.push('supprimé en local (' + (fmtSize(dl.freed) || '0 o') + ')');
      else toast('Suppression locale échouée : ' + (dl.error || ''), 'err');
    }
    // 3) Suppression Drive si demandée
    if (choice === 'drive' || choice === 'both') {
      let rd = await fetch('/api/drive_delete', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ path: _exc.path, is_dir: _exc.isDir }) });
      let dd = await rd.json();
      if (dd.ok) msgs.push('supprimé du Drive');
      else toast('Suppression Drive échouée : ' + (dd.error || ''), 'err');
    }
    toast(msgs.join(' · '), 'ok');
    closeExcludeModal();
    if (document.getElementById('tree-modal').classList.contains('show')) loadTree(_fm.dir);
  } catch (e) {
    toast('Serveur injoignable', 'err');
    btn.disabled = false;
    btn.textContent = origLabel;
  }
}

export function closeExcludeModal() {
  document.getElementById('exclude-modal').classList.remove('show');
}

/* Ré-inclure : ne retire QUE la règle exacte de l'élément. Si un motif global
   (ex. « out/** ») est en cause, on ne retire rien et on explique lequel. */
export async function reincludePath(path, isDir) {
  let exact = isDir ? path + '/**' : path;   // motif « nu » (sans « - »)
  try {
    let r = await fetch('/api/match_rules?path=' + encodeURIComponent(path));
    let d = await r.json();
    if (!d.ok) { toast('Erreur : ' + (d.error || ''), 'err'); return; }
    let rules = d.rules || [];
    if (rules.length === 0) {
      // plus aucun motif ne matche (déjà ré-inclus entre-temps)
      toast('Cet élément n\'est plus exclu', 'ok');
      loadTree(_fm.dir);
      return;
    }
    let others = rules.filter(function (x) { return x !== exact; });
    if (others.length === 0) {
      // seule la règle exacte de l'élément l'exclut → retrait sûr
      let rr = await fetch('/api/filters_remove?rule=' + encodeURIComponent('- ' + exact));
      let dr = await rr.json();
      if (dr.ok && dr.removed) {
        toast('Ré-inclus dans la synchronisation', 'ok');
        loadTree(_fm.dir);
      } else {
        toast('Impossible de retirer la règle', 'err');
      }
      return;
    }
    // un ou plusieurs motifs globaux s'appliquent → popup explicatif, sans rien retirer
    openReincModal(path, rules);
  } catch (e) {
    toast('Serveur injoignable', 'err');
  }
}

export function openReincModal(path, rules) {
  document.getElementById('reinc-modal').classList.add('show');
  document.getElementById('reinc-summary').innerHTML =
    '<div class="del-banner warn">' + ICO_WARN
    + '<div><b>' + esc(path) + '</b> n\'est pas exclu par une règle qui lui est propre, mais par '
    + (rules.length > 1 ? 'ces motifs généraux' : 'ce motif général') + ' :</div></div>';
  let html = '';
  rules.forEach(function (rp) {
    html += '<div class="del-file"><span class="df-name">- ' + esc(rp) + '</span></div>';
  });
  document.getElementById('reinc-rules').innerHTML = html;
}
export function closeReincModal() {
  document.getElementById('reinc-modal').classList.remove('show');
}
