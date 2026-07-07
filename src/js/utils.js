/* ═══════════════════════════════════════════════════
   UTILS
   ═══════════════════════════════════════════════════ */
export function esc(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

export function fmtT(ts) {
  if (!ts) return '—';
  try {
    return new Date(ts).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return ts.slice(11, 19) || ts;
  }
}

export function fmtDT(ts) {
  if (!ts) return '—';
  let d = new Date(ts);
  if (isNaN(d)) return ts.slice(0, 16);
  let now = new Date();
  let yest = new Date(now); yest.setDate(now.getDate() - 1);
  let hm = d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
  if (d.toDateString() === now.toDateString()) return "Auj. " + hm;
  if (d.toDateString() === yest.toDateString()) return 'Hier ' + hm;
  return d.toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' }) + ' ' + hm;
}

export function spin(v) { document.getElementById('spin').classList.toggle('on', v); }

export function fmtSize(bytes) {
  if (bytes == null || isNaN(bytes)) return '';
  if (bytes < 1024) return bytes + ' o';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' Ko';
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' Mo';
  if (bytes < 1024 ** 4) return (bytes / (1024 ** 3)).toFixed(1) + ' Go';
  return (bytes / (1024 ** 4)).toFixed(2) + ' To';
}

/* "2m9.5s" / "1h2m3s" / "45.6s" → secondes */
export function parseElapsed(e) {
  if (!e) return 0;
  let s = 0, m;
  if ((m = e.match(/(\d+)h/))) s += parseInt(m[1]) * 3600;
  if ((m = e.match(/(\d+)m(?!s)/))) s += parseInt(m[1]) * 60;
  if ((m = e.match(/([\d.]+)s/))) s += parseFloat(m[1]);
  return s;
}

export function fmtRemaining(s) {
  if (s <= 0) return 'imminente…';
  if (s < 60) return Math.round(s) + ' s';
  if (s < 3600) return Math.floor(s / 60) + ' min ' + String(Math.round(s % 60)).padStart(2, '0') + ' s';
  return Math.floor(s / 3600) + ' h ' + String(Math.floor((s % 3600) / 60)).padStart(2, '0') + ' min';
}

/* Extracted utils */
export function colorizeLog(text) {
  let e = esc(text);
  // Préfixe journalctl « <ISO> <hôte> <unité[pid]>: » → ne garder que l'heure
  // lisible et estomper hôte + unité, pour que le message ressorte.
  e = e.replace(
    /^\d{4}[-\/]\d{2}[-\/]\d{2}[T ](\d{2}:\d{2}:\d{2})[^\s]*\s+(\S+)\s+(\S+?):/,
    '<span class="log-meta">$1</span> <span class="log-meta">$2 $3:</span>'
  );
  // Repli : si le préfixe complet n'est pas reconnu, estomper au moins l'horodatage.
  if (e.indexOf('log-meta') === -1) {
    e = e.replace(/^(\d{4}[-\/]\d{2}[-\/]\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*)/, '<span class="log-meta">$1</span>');
  }
  e = e.replace(/ ERROR /g, ' <strong style="color:var(--err)">ERROR </strong>');
  e = e.replace(/ NOTICE(:| )/g, ' <strong style="color:var(--warn)">NOTICE</strong>$1');
  e = e.replace(/ INFO(:| )/g, ' <strong style="color:var(--run)">INFO  </strong>$1');
  e = e.replace(/ DEBUG(:| )/g, ' <strong style="color:var(--faint)">DEBUG </strong>$1');
  e = e.replace(/(Deleted .*|File was deleted.*)/g, '<span style="color:var(--err)">$1</span>');
  e = e.replace(/(Copied .*|File is new.*)/g, '<span style="color:var(--ok)">$1</span>');
  e = e.replace(/(Updated .*|File was modified.*)/g, '<span style="color:var(--warn)">$1</span>');
  return e;
}

/* Génération HTML unifiée pour les lignes de fichiers récents et en cours */
export function renderFileRow(f, extraStyle, opts) {
  opts = opts || {};
  let cls = f.action || 'new';
  let sizeTxt = opts.customSize || fmtSize(f.size);
  let labels = { 'new': 'Copié', 'modified': 'Modifié', 'deleted': 'Supprimé', 'excluded': 'Exclu' };
  
  let timeTxt = '';
  if (f.time && !opts.hideTime) {
    let t = String(f.time);
    // Horodatage ISO complet → format court cohérent avec l'historique
    // (« Auj. 18:19 ») ; heure déjà courte → telle quelle.
    timeTxt = /\d{4}-\d{2}-\d{2}/.test(t) ? fmtDT(t)
            : (t.indexOf(':') !== -1 ? esc(t) : fmtT(t));
  }
  
  let styleAttr = extraStyle ? ' style="' + extraStyle + '"' : '';
  
  // Délégation d'événements : plus de onclick=openFile('...') à échapper à la
  // main. Le chemin voyage dans un data-attribut (esc gère les guillemets) et un
  // unique écouteur délégué (main.js) déclenche l'ouverture.
  let openAttrs = ' data-openfile="1" data-fpath="' + esc(f.path) + '" data-deleted="' + (cls === 'deleted' ? '1' : '0') + '"';
  let html = '<div class="recent-item file-link"' + openAttrs + ' title="Ouvrir le fichier — Ctrl+clic pour ouvrir son dossier"' + styleAttr + '>';
  
  if (!opts.hideAction) {
    html += '<span class="recent-dot ' + cls + '"></span>'
         + '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + '</span>';
  }
  
  let namePrefix = f.is_dir ? '📁 ' : '';
  html += '<span class="recent-path" title="' + esc(f.path) + '">' + namePrefix + esc(f.path) + '</span>';
  if (sizeTxt) html += '<span class="recent-size">' + sizeTxt + '</span>';
  if (timeTxt) html += '<span class="recent-time">' + timeTxt + '</span>';
  
  html += '</div>';
  return html;
}
