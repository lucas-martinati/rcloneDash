import os
import json

PORT = 8765

def get_settings():
    try:
        with open(os.path.expanduser("~/.config/rclone/dash-config.json"), "r") as f:
            return json.load(f)
    except Exception:
        return {"remote": "GoogleDrive:", "local_dir": "~/GoogleDrive", "timer_interval": "10min"}

_settings = get_settings()
REMOTE = _settings.get("remote", "GoogleDrive:")
GD_DIR = os.path.expanduser(_settings.get("local_dir", "~/GoogleDrive"))

FILTERS_PATH = os.path.expanduser("~/.config/rclone/gdrive-filters.txt")
BWLIMIT_PATH = os.path.expanduser("~/.config/rclone/bwlimit.env")

PHASES = [
    "Syncing Path1",
    "Syncing Path2",
    "Checking Path1",
    "Checking Path2",
    "Updating Path1",
    "Updating Path2",
    "Queued",
    "Done",
]
