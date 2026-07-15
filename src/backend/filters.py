import os
import re
from . import config


def local_size(fpath):
    """Taille du fichier local (en octets), ou None si introuvable."""
    try:
        return os.path.getsize(os.path.join(config.GD_DIR, fpath))
    except OSError:
        return None


def load_exclude_rules():
    """Renvoie la liste des motifs d'exclusion (lignes « - motif ») du fichier de filtres."""
    rules = []
    try:
        with open(config.FILTERS_PATH, "r") as f:
            for line in f:
                line = line.strip()
                if line.startswith("- "):
                    rules.append(line[2:].strip())
    except OSError:
        pass
    return rules


def _glob_to_regex(pat):
    """Convertit un motif glob rclone (*, **, ?) en fragment d'expression régulière.

    « **/ » est traité comme un préfixe de chemin optionnel (rclone : `**` peut
    matcher zéro segment), pour que « **/Thumbs.db » attrape aussi la racine.
    """
    out = []
    i = 0
    n = len(pat)
    while i < n:
        if pat[i : i + 3] == "**/":
            out.append("(?:.*/)?")
            i += 3
        elif pat[i : i + 2] == "**":
            out.append(".*")
            i += 2
        elif pat[i] == "*":
            out.append("[^/]*")
            i += 1
        elif pat[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(pat[i]))
            i += 1
    return "".join(out)


def _match_any_level(rx, path, anchored):
    """Teste un motif contre le chemin, ancré à la racine ou à n'importe quel « / ».

    rclone : un motif sans « / » initial se compare à la fin du chemin, à
    n'importe quel niveau ; un motif commençant par « / » est ancré à la racine.
    """
    segs = path.split("/")
    starts = [0] if anchored else range(len(segs))
    for i in starts:
        if re.fullmatch(rx, "/".join(segs[i:])):
            return True
    return False


def path_is_ignored(rel, rules):
    """Approxime le matching des filtres rclone pour un chemin relatif donné.

    Couvre les motifs du fichier : « dossier/** » (à n'importe quel niveau),
    « *.doc », « **/.DS_Store », noms nus, et les règles ancrées « /a/b/** ».
    """
    rel = rel.replace(os.sep, "/").strip("/")
    if not rel:
        return False
    for pat in rules:
        anchored = pat.startswith("/")
        body = pat[1:] if anchored else pat
        if not body:
            continue
        rx_full = _glob_to_regex(body)
        if _match_any_level(rx_full, rel, anchored):
            return True
        # « base/** » : le dossier « base » lui-même est aussi considéré exclu
        if body.endswith("/**"):
            rx_base = _glob_to_regex(body[:-3])
            if _match_any_level(rx_base, rel, anchored):
                return True
    return False
