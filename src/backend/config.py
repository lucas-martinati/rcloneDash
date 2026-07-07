import os

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
