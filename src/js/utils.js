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
  var d = new Date(ts);
  if (isNaN(d)) return ts.slice(0, 16);
  var now = new Date();
  var yest = new Date(now); yest.setDate(now.getDate() - 1);
  var hm = d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
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
  var s = 0, m;
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
  var e = esc(text);
  e = e.replace(/^(\d{4}[-\/]\d{2}[-\/]\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*)/gm, '<span style="opacity:0.5">$1</span>');
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
  var cls = f.action || 'new';
  var pathArg = esc(f.path).replace(/\\/g, '\\\\').replace(/'/g, "\\'");
  var actionCall = "openFile('" + pathArg + "', " + (cls === 'deleted' ? 'true' : 'false') + ", event)";
  var sizeTxt = opts.customSize || fmtSize(f.size);
  var labels = { 'new': 'Copié', 'modified': 'Modifié', 'deleted': 'Supprimé', 'excluded': 'Exclu' };
  
  var timeTxt = '';
  if (f.time && !opts.hideTime) {
    timeTxt = String(f.time).indexOf(':') !== -1 ? esc(f.time) : fmtT(f.time);
  }
  
  var styleAttr = extraStyle ? ' style="' + extraStyle + '"' : '';
  
  var html = '<div class="recent-item file-link" onclick="' + actionCall + '" title="Ouvrir le fichier — Ctrl+clic pour ouvrir son dossier"' + styleAttr + '>';
  
  if (!opts.hideAction) {
    html += '<span class="recent-dot ' + cls + '"></span>'
         + '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + '</span>';
  }
  
  var namePrefix = f.is_dir ? '📁 ' : '';
  html += '<span class="recent-path" title="' + esc(f.path) + '">' + namePrefix + esc(f.path) + '</span>';
  if (sizeTxt) html += '<span class="recent-size">' + sizeTxt + '</span>';
  if (timeTxt) html += '<span class="recent-time">' + timeTxt + '</span>';
  
  html += '</div>';
  return html;
}
