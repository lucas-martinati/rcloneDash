/* ═══════════════════════════════════════════════════
   ICONS (Extracted)
   ═══════════════════════════════════════════════════ */
export function _svg(inner, sz) {
  sz = sz || 14;
  return (
    '<svg width="' +
    sz +
    '" height="' +
    sz +
    '" viewBox="0 0 24 24" fill="none" ' +
    'stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
    inner +
    '</svg>'
  );
}
export const FM_ICONS = {
  folder: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>',
  image:
    '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>',
  video:
    '<polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2"/>',
  audio: '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>',
  archive:
    '<polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/>',
  doc: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>',
  sheet:
    '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="12" y1="11" x2="12" y2="19"/>',
  slides:
    '<rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>',
  pdf: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M9 13h1.5a1 1 0 0 1 0 3H9zM9 13v6"/>',
  code: '<polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>',
  file: '<path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/>'
};
export const FM_EXT = {
  image: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'heic', 'heif', 'tiff', 'ico', 'avif'],
  video: ['mp4', 'mkv', 'avi', 'mov', 'webm', 'wmv', 'flv', 'm4v', 'mpg', 'mpeg', '3gp'],
  audio: ['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a', 'wma', 'opus', 'aiff'],
  archive: ['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'tgz', 'zst'],
  pdf: ['pdf'],
  doc: ['doc', 'docx', 'odt', 'rtf', 'txt', 'md', 'pages', 'tex', 'epub'],
  sheet: ['xls', 'xlsx', 'ods', 'csv', 'tsv', 'numbers'],
  slides: ['ppt', 'pptx', 'odp', 'key'],
  code: [
    'js',
    'ts',
    'jsx',
    'tsx',
    'py',
    'rb',
    'go',
    'rs',
    'c',
    'cpp',
    'h',
    'hpp',
    'java',
    'php',
    'html',
    'htm',
    'css',
    'scss',
    'sass',
    'json',
    'xml',
    'yml',
    'yaml',
    'sh',
    'bash',
    'zsh',
    'sql',
    'swift',
    'kt',
    'lua',
    'vue',
    'ini',
    'toml'
  ]
};
export function fmCategory(item) {
  if (item.is_dir) return 'folder';
  let dot = item.name.lastIndexOf('.');
  if (dot < 1) return 'file';
  let ext = item.name.slice(dot + 1).toLowerCase();
  for (let cat in FM_EXT) if (FM_EXT[cat].indexOf(ext) !== -1) return cat;
  return 'file';
}
export const ICO_CHECK = _svg('<path d="M20 6L9 17l-5-5"/>', 16);
export const ICO_WARN = _svg(
  '<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>',
  16
);
export const ICO_CLOUD = _svg(
  '<path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/><polyline points="9 15 11 17 15 12"/>',
  15
);
