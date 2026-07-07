import re

in_file = 'src/rclone-monitor.py'
with open(in_file, 'r', encoding='utf-8') as f:
    lines = f.readlines()

hand_code = ["import http.server\n", "import json\n", "import os\n", "import subprocess\n", "import shutil\n", "import re\n", "import urllib.parse\n", "from urllib.parse import urlparse, parse_qs\n", "from . import config\n", "from .filters import load_exclude_rules, path_is_ignored\n\n"]
hand_str = "".join(lines[755:1324])
hand_str = hand_str.replace("import re", "")
hand_str = hand_str.replace("import glob, shutil, re", "import glob")
hand_str = hand_str.replace("FILTERS_PATH", "config.FILTERS_PATH")
hand_str = hand_str.replace("GD_DIR", "config.GD_DIR")
# Replace whole words \b_m\b and \b_d\b
hand_str = re.sub(r'\b_m\b', 'self._m', hand_str)
hand_str = re.sub(r'\b_d\b', 'self._d', hand_str)

hand_code.append(hand_str)
with open('src/backend/handler.py', 'w', encoding='utf-8') as f:
    f.write("".join(hand_code))
print("Fixed handler.py")
