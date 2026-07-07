import { ICO_CHECK } from './icons.js';
import { esc, fmtSize, renderFileRow, colorizeLog } from './utils.js';
import { toast } from './toasts.js';
import { loadTree, _fm } from './file-browser.js';

/* ═══════════════════════════════════════════════════
   EXCLUSIONS (filtres rclone)
   ═══════════════════════════════════════════════════ */
export async function openFiltersModal() {
  document.getElementById('filters-modal').classList.add('show');
  await loadFilters();
}

export function closeFiltersModal() { document.getElementById('filters-modal').classList.remove('show'); }

export let _originalFiltersText = '';

export function checkFiltersModified() {
  var tf = document.getElementById('filters-text');
  var btn = document.getElementById('save-filters-btn');
  btn.disabled = tf.value === _originalFiltersText;
}
export async function loadFilters() {
  var tf = document.getElementById('filters-text');
  tf.value = 'Chargement…';
  try {
    var r = await fetch('/api/filters');
    var d = await r.json();
    tf.value = d.content || (d.error ? 'Erreur : ' + d.error : '');
    _originalFiltersText = tf.value;
    checkFiltersModified();
    tf.scrollTop = tf.scrollHeight;
  } catch (e) {
    tf.value = 'Serveur injoignable';
  }
}

export function addFilter() {
  var input = document.getElementById('new-filter-input');
  var rule = input.value.trim();
  if (!rule) return;
  if (!rule.startsWith('- ') && !rule.startsWith('+ ') && !rule.startsWith('#')) {
    rule = '- ' + rule;
  }
  // commentaires et inclusions (« + ») n'excluent rien → ajout direct
  if (rule.startsWith('#') || rule.startsWith('+ ')) { commitFilter(rule); return; }
  // règle d'exclusion → on montre d'abord tout ce qu'elle affecte
  openImpactModal(rule);
}

export function commitFilter(rule) {
  var input = document.getElementById('new-filter-input');
  var tf = document.getElementById('filters-text');
  tf.value += (tf.value.endsWith('\n') || !tf.value ? '' : '\n') + rule + '\n';
  input.value = '';
  tf.scrollTop = tf.scrollHeight;
  return saveFilters();
}

export let _pendingFilter = null;
export function openImpactModal(rule) {
  _pendingFilter = rule;
  document.getElementById('impact-modal').classList.add('show');
  document.getElementById('impact-rule').textContent = rule;
  var noneRadio = document.querySelector('input[name="imp-action"][value="none"]');
  if (noneRadio) noneRadio.checked = true;
  impOnChoice();
  document.getElementById('impact-summary').innerHTML = '<div class="del-loading">Analyse de l\'impact…</div>';
  document.getElementById('impact-files').innerHTML = '';
  document.getElementById('impact-confirm').disabled = true;
  fetch('/api/rule_impact?rule=' + encodeURIComponent(rule)).then(function (r) { return r.json(); }).then(function (d) {
    if (!d.ok) {
      document.getElementById('impact-summary').innerHTML = '<div class="del-count">Impact non calculable : ' + esc(d.error || '') + '</div>';
      document.getElementById('impact-confirm').disabled = false;
      return;
    }
    var summary;
    if (d.count === 0) {
      summary = '<div class="del-banner ok">' + ICO_CHECK + '<div>Aucun élément présent ne correspond pour l\'instant. La règle s\'appliquera aux futurs fichiers.</div></div>';
    } else {
      var noun = d.count > 1 ? 'éléments' : 'élément';
      summary = '<div class="del-count">Cette règle exclut <span class="del-big2">' + d.count + '</span> ' + noun
        + ' · <b>' + esc(fmtSize(d.size) || '0 o') + '</b>' + (d.truncated ? ' (aperçu partiel)' : '') + '</div>';
    }
    document.getElementById('impact-summary').innerHTML = summary;
    var fl = '';
    d.items.forEach(function (it) {
      var meta = it.is_dir ? ((it.count || 0) + (it.count > 1 ? ' fichiers' : ' fichier')) : (fmtSize(it.size) || '');
      it.action = 'excluded';
      fl += renderFileRow(it, '', { hideTime: true, customSize: meta, hideAction: true });
    });
    if (d.truncated) fl += '<div class="del-more">… et d\'autres éléments non listés</div>';
    document.getElementById('impact-files').innerHTML = fl;
    document.getElementById('impact-confirm').disabled = false;
  }).catch(function () {
    document.getElementById('impact-summary').innerHTML = '<div class="del-count">Serveur injoignable</div>';
    document.getElementById('impact-confirm').disabled = false;
  });
}
export function closeImpactModal() {
  document.getElementById('impact-modal').classList.remove('show');
  _pendingFilter = null;
}
export function impChoice() {
  var el = document.querySelector('input[name="imp-action"]:checked');
  return el ? el.value : 'none';
}
export function impOnChoice() {
  var v = impChoice();
  var btn = document.getElementById('impact-confirm');
  var labels = {
    none: 'Ajouter cette exclusion',
    local: 'Exclure et supprimer en local',
    drive: 'Exclure et supprimer du Drive',
    both: 'Exclure et supprimer des deux'
  };
  btn.textContent = labels[v] || 'Ajouter cette exclusion';
  btn.className = 'btn ' + (v === 'none' ? 'btn-g' : 'btn-danger');
}
export async function confirmAddFilter() {
  var rule = _pendingFilter;
  if (!rule) { closeImpactModal(); return; }
  var choice = impChoice();
  var btn = document.getElementById('impact-confirm');
  btn.disabled = true;
  btn.textContent = 'En cours…';
  try {
    // 1) Ajouter la règle d'abord (empêche toute propagation bisync des suppressions)
    await commitFilter(rule);
    // 2) Suppression de tout ce que la règle couvre, si demandé
    if (choice !== 'none') {
      var r = await fetch('/api/rule_delete', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ rule: rule, mode: choice })
      });
      var d = await r.json();
      if (d.ok) {
        var parts = [d.count + ' élément' + (d.count > 1 ? 's' : '') + ' traité' + (d.count > 1 ? 's' : '')];
        if (choice !== 'drive') parts.push((fmtSize(d.freed) || '0 o') + ' libérés');
        if (d.truncated) parts.push('liste tronquée');
        var hasErr = d.errors && d.errors.length;
        if (hasErr) parts.push(d.errors.length + ' erreur(s)');
        toast('Exclusion appliquée · ' + parts.join(' · '), hasErr ? 'warn' : 'ok');
      } else {
        toast('Suppression échouée : ' + (d.error || ''), 'err');
      }
    }
    closeImpactModal();
    if (document.getElementById('tree-modal').classList.contains('show')) loadTree(_fm.dir);
  } catch (e) {
    toast('Serveur injoignable', 'err');
    btn.disabled = false;
    impOnChoice();
  }
}

export async function saveFilters() {
  var tf = document.getElementById('filters-text');
  var btn = document.getElementById('save-filters-btn');
  btn.disabled = true;
  btn.textContent = 'Enregistrement…';
  try {
    var r = await fetch('/api/filters_save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content: tf.value })
    });
    var d = await r.json();
    if (d.ok) {
      toast('Exclusions enregistrées', 'ok');
      _originalFiltersText = tf.value;
      checkFiltersModified();
    } else {
      toast('Enregistrement impossible : ' + d.error, 'err');
    }
  } catch (e) {
    toast('Serveur injoignable — exclusions non enregistrées', 'err');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Enregistrer';
  }
}
