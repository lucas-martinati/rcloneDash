import os
import re

app_js_path = 'src/app.js'
out_dir = 'src/js'
os.makedirs(out_dir, exist_ok=True)

with open(app_js_path, 'r', encoding='utf-8') as f:
    content = f.read()

blocks = re.split(r'(?=\/\*\s*[═]+\n)', content)
blocks = [b for b in blocks if b.strip()]

files = {
    '00-state.js': '',
    '01-utils.js': '',
    '02-toasts.js': '',
    '03-theme.js': '',
    '04-status.js': '',
    '05-pulse.js': '',
    '06-dashboard.js': '',
    '07-live-sync.js': '',
    '08-sparkline.js': '',
    '09-history.js': '',
    '10-logs.js': '',
    '11-recent.js': '',
    '12-drag-resize.js': '',
    '13-refresh.js': '',
    '14-icons.js': '',
    '15-file-browser.js': '',
    '16-exclude.js': '',
    '17-delete.js': '',
    '18-filters.js': '',
    '19-modals.js': '',
    '20-init.js': '',
    '99-unknown.js': '',
}

def add_to_file(filename, text):
    if files[filename]:
        files[filename] += '\n' + text.strip() + '\n'
    else:
        files[filename] = text.strip() + '\n'

colorize_log_code = ""
icons_code = ""

for b in blocks:
    m = re.match(r'\/\*\s*[═]+\n\s*(.*?)\n', b)
    title = m.group(1).strip() if m else "UNKNOWN"
    
    if title.startswith('STATE'): add_to_file('00-state.js', b)
    elif title.startswith('UTILS'): add_to_file('01-utils.js', b)
    elif title.startswith('TOASTS'): add_to_file('02-toasts.js', b)
    elif title.startswith('THÈME'): add_to_file('03-theme.js', b)
    elif title.startswith('BADGE'): add_to_file('04-status.js', b)
    elif title.startswith('AUTO-REFRESH'): add_to_file('04-status.js', b)
    elif title.startswith('POULS'): add_to_file('05-pulse.js', b)
    elif title.startswith('QUOTA'): add_to_file('06-dashboard.js', b)
    elif title.startswith('ALERTES'): add_to_file('06-dashboard.js', b)
    elif title.startswith('KPIs'): add_to_file('06-dashboard.js', b)
    elif title.startswith('SYNC EN COURS'): add_to_file('07-live-sync.js', b)
    elif title.startswith('SPARKLINE'): add_to_file('08-sparkline.js', b)
    elif title.startswith('HISTORIQUE'): add_to_file('09-history.js', b)
    elif title.startswith('JOURNAL'): add_to_file('10-logs.js', b)
    elif title.startswith('FICHIERS RÉCENTS'): add_to_file('11-recent.js', b)
    elif title.startswith('MODULES DÉPLAÇABLES'): add_to_file('12-drag-resize.js', b)
    elif title.startswith('REFRESH PRINCIPAL'): add_to_file('13-refresh.js', b)
    elif title.startswith('ACTIONS SYNC'): add_to_file('13-refresh.js', b)
    elif title.startswith('NAVIGATEUR DE FICHIERS'):
        lines = b.split('\n')
        browser_lines = []
        in_icon = False
        icon_buffer = []
        for line in lines:
            if line.startswith('function _svg(') or line.startswith('const FM_ICONS') or line.startswith('const FM_EXT') or line.startswith('function fmCategory('):
                in_icon = True
                icon_buffer.append(line)
            elif in_icon:
                icon_buffer.append(line)
                if line.startswith('}'):
                    in_icon = False
                    icons_code += '\n'.join(icon_buffer) + '\n'
                    icon_buffer = []
            else:
                browser_lines.append(line)
        add_to_file('15-file-browser.js', '\n'.join(browser_lines))
    elif title.startswith('EXCLUSIONS (filtres'):
        lines = b.split('\n')
        filt_lines = []
        in_color = False
        color_buffer = []
        for line in lines:
            if line.startswith('function colorizeLog('):
                in_color = True
                color_buffer.append(line)
            elif in_color:
                color_buffer.append(line)
                if line.startswith('}'):
                    in_color = False
                    colorize_log_code += '\n'.join(color_buffer) + '\n'
                    color_buffer = []
            else:
                filt_lines.append(line)
        add_to_file('18-filters.js', '\n'.join(filt_lines))
    elif title.startswith('EXCLUSION'): add_to_file('16-exclude.js', b)
    elif title.startswith('SUPPRESSION LOCALE'):
        lines = b.split('\n')
        del_lines = []
        for line in lines:
            if line.startswith('const ICO_CHECK') or line.startswith('const ICO_WARN') or line.startswith('const ICO_CLOUD'):
                icons_code += line + '\n'
            else:
                del_lines.append(line)
        add_to_file('17-delete.js', '\n'.join(del_lines))

    elif title.startswith('LIMITE DE DÉBIT'): add_to_file('19-modals.js', b)
    elif title.startswith('SIMULATION'): add_to_file('19-modals.js', b)
    elif title.startswith('CLAVIER'): add_to_file('20-init.js', b)
    elif title.startswith('CTRL MAINTENU'): add_to_file('20-init.js', b)
    elif title.startswith('BOOT'): add_to_file('20-init.js', b)
    else:
        print(f"Unknown title: {title}")
        add_to_file('99-unknown.js', b)

if colorize_log_code:
    files['01-utils.js'] += '\n/* Extracted utils */\n' + colorize_log_code.strip() + '\n'
if icons_code:
    files['14-icons.js'] = '/* ═══════════════════════════════════════════════════\n   ICONS (Extracted)\n   ═══════════════════════════════════════════════════ */\n' + icons_code.strip() + '\n'

for filename, content in files.items():
    if content:
        with open(os.path.join(out_dir, filename), 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Created {filename}")
