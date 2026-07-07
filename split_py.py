import os
import re

in_file = 'src/rclone-monitor.py'
with open(in_file, 'r', encoding='utf-8') as f:
    lines = f.readlines()

out_dir = 'src/backend'
os.makedirs(out_dir, exist_ok=True)

def write_file(name, content):
    with open(os.path.join(out_dir, name), 'w', encoding='utf-8') as f:
        f.write(content)

# __init__.py
write_file('__init__.py', '')

# config.py
config_content = """import os

PORT = 8765
GD_DIR = os.path.expanduser("~/GoogleDrive")
FILTERS_PATH = os.path.expanduser("~/.config/rclone/gdrive-filters.txt")

PHASES = [
    "Syncing Path1",
    "Syncing Path2",
    "Checking Path1",
    "Checking Path2",
    "Updating Path1",
    "Updating Path2",
    "Queued",
    "Done"
]
"""
write_file('config.py', config_content)

# filters.py
filters_code = ["import os", "import re"]
filters_code.extend(lines[34:41])
filters_code.extend(lines[45:125])
write_file('filters.py', "".join(filters_code).replace('FILTERS_PATH', 'config.FILTERS_PATH').replace('GD_DIR', 'config.GD_DIR'))
# add config import
with open(os.path.join(out_dir, 'filters.py'), 'r', encoding='utf-8') as f:
    fc = f.read()
fc = fc.replace("import re\n", "import re\nfrom . import config\n")
write_file('filters.py', fc)

# log_streamer.py
ls_code = ["import threading\n", "import subprocess\n", "import re\n", "import time\n", "from collections import deque\n", "from datetime import datetime\n", "from . import config\n", "from .filters import local_size\n\n"]
ls_code.extend(lines[136:380])
write_file('log_streamer.py', "".join(ls_code).replace('PHASES', 'config.PHASES').replace('GD_DIR', 'config.GD_DIR'))

# monitor.py
mon_code = ["import threading\n", "import subprocess\n", "import json\n", "import os\n", "import shutil\n", "import re\n", "import time\n", "from datetime import datetime\n", "from . import config\n", "from .filters import local_size\n", "from .log_streamer import LogStreamer\n\n"]
mon_code.extend(lines[381:750])
write_file('monitor.py', "".join(mon_code).replace('GD_DIR', 'config.GD_DIR'))

# server.py
# ThreadingServer + main
srv_code = """import os
import threading
import socketserver
from . import config
from .monitor import Monitor
from .handler import Handler

class ThreadingServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    daemon_threads = True

def main():
    Handler._m = Monitor()
    Handler._d = os.path.dirname(os.path.abspath(__file__))
    Handler._d = os.path.dirname(Handler._d) # Go up one level to src

    threading.Thread(target=Handler._m.update_quota, daemon=True).start()
    print(f"\\033[1;36mRcloneDash\\033[0m — Démarrage du serveur sur le port {config.PORT}")
    try:
        with ThreadingServer(("", config.PORT), Handler) as httpd:
            httpd.serve_forever()
    except KeyboardInterrupt:
        pass
"""
write_file('server.py', srv_code)

# handler.py
# For handler.py, we need to extract routes. We'll do a simple regex or just string manipulation.
# For now, let's dump the Handler class, and fix redundant imports.
hand_code = ["import http.server\n", "import json\n", "import os\n", "import subprocess\n", "import shutil\n", "import re\n", "import urllib.parse\n", "from urllib.parse import urlparse, parse_qs\n", "from . import config\n", "from .filters import load_exclude_rules, path_is_ignored\n\n"]
hand_str = "".join(lines[755:1324])
hand_str = hand_str.replace("import re", "")
hand_str = hand_str.replace("import glob, shutil, re", "import glob")
hand_str = hand_str.replace("FILTERS_PATH", "config.FILTERS_PATH")
hand_str = hand_str.replace("GD_DIR", "config.GD_DIR")
hand_str = hand_str.replace("_m.", "self._m.")
hand_str = hand_str.replace("_m", "self._m")
hand_str = hand_str.replace("_d", "self._d")
hand_code.append(hand_str)
write_file('handler.py', "".join(hand_code))

print("Created Python files in src/backend/")
