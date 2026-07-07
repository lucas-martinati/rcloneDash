import os

PORT = 8765
# Remote rclone (avec le « : » final) et dossier local synchronisé.
# Tout est centralisé ici : pour changer de remote ou de dossier, une seule
# ligne à éditer plutôt que la vingtaine de chemins codés en dur d'avant.
REMOTE = "GoogleDrive:"
GD_DIR = os.path.expanduser("~/GoogleDrive")
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
    "Done"
]
