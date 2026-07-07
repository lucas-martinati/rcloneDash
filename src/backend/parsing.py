"""Parsing partagé des lignes de log rclone.

Centralise ce qui était dupliqué à l'identique entre `monitor.py` (historique)
et `log_streamer.py` (live) : le motif d'une ligne « fichier synchronisé » et la
correspondance action rclone → catégorie. Toute évolution du format de log
rclone se répercute désormais en un seul endroit.
"""
import re

# INFO  : chemin/du/fichier: Copied (new)  /  Deleted  /  Updated file …
_SYNCED_FILE_RE = re.compile(
    r"INFO\s+:\s+(.*?):\s+"
    r"(Copied \(new\)|Copied \(replaced existing\)"
    r"|Updated modification time in destination|Deleted|Updated file)"
)


def parse_synced_file(line):
    """Extrait (chemin, catégorie) d'une ligne de fichier synchronisé, ou None.

    catégorie ∈ {"new", "modified", "deleted"} — vocabulaire commun au front.
    """
    m = _SYNCED_FILE_RE.search(line)
    if not m:
        return None
    fpath = m.group(1).strip()
    action = m.group(2).strip()
    if "Copied (new)" in action:
        category = "new"
    elif "Deleted" in action:
        category = "deleted"
    else:  # Copied (replaced existing) | Updated modification | Updated file
        category = "modified"
    return fpath, category
