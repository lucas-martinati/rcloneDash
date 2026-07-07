import http.server
import json
import os
import subprocess
import shutil
import re
import urllib.parse
from urllib.parse import urlparse, parse_qs
from . import config
from .filters import load_exclude_rules, path_is_ignored

class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_GET(self):
        p = urlparse(self.path).path
        if p == "/api/status":
            self._json(self._m.full())
        elif p == "/api/trigger":
            ok, err = self._m.trigger()
            self._json({"ok": ok, "error": err})
        elif p == "/api/cancel":
            ok, err = self._m.cancel()
            self._json({"ok": ok, "error": err})
        elif p == "/api/bwlimit":
            try:
                env_path = os.path.expanduser("~/.config/rclone/bwlimit.env")
                limit = ""
                if os.path.exists(env_path):
                    with open(env_path, "r") as f:
                        content = f.read()
                        
                        m = re.search(r"RCLONE_BWLIMIT=(.*)", content)
                        if m: limit = m.group(1).strip()
                self._json({"limit": limit})
            except Exception as e:
                self._json({"error": str(e)})
        elif p == "/api/bwlimit_save":
            qs = parse_qs(urlparse(self.path).query)
            limit = qs.get("limit", [""])[0]
            try:
                env_path = os.path.expanduser("~/.config/rclone/bwlimit.env")
                os.makedirs(os.path.dirname(env_path), exist_ok=True)
                with open(env_path, "w") as f:
                    if limit:
                        f.write(f"RCLONE_BWLIMIT={limit}\n")
                    else:
                        f.write("")
                self._json({"ok": True})
            except Exception as e:
                self._json({"error": str(e)})
        elif p == "/api/dryrun":
            try:
                # Copie des listes réelles vers les listes -dry pour refléter l'état actuel
                import glob
                cache_dir = os.path.expanduser("~/.cache/rclone/bisync")
                if os.path.exists(cache_dir):
                    for f in glob.glob(os.path.join(cache_dir, "*.lst")):
                        try: shutil.copy2(f, f + "-dry")
                        except Exception: pass
                        
                cmd = ["rclone", "bisync", "GoogleDrive:", os.path.expanduser("~/GoogleDrive"), "--dry-run", "-v", "--tpslimit", "8", "--filter-from", os.path.expanduser("~/.config/rclone/gdrive-filters.txt")]
                out = subprocess.check_output(cmd, stderr=subprocess.STDOUT, timeout=600).decode('utf-8', errors='ignore')
                out = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', out)
                self._json({"ok": True, "log": out})
            except subprocess.CalledProcessError as e:
                out = e.output.decode('utf-8', errors='ignore')
                
                out = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', out)
                self._json({"ok": False, "error": out})
            except Exception as e:
                self._json({"ok": False, "error": str(e)})
        elif p == "/api/tree":
            qs = parse_qs(urlparse(self.path).query)
            target = qs.get("dir", [""])[0]
            base = os.path.abspath(os.path.expanduser("~/GoogleDrive"))
            target_path = os.path.abspath(os.path.join(base, target))
            if not target_path.startswith(base):
                self._json({"error": "Invalid path"})
                return
            try:
                if not os.path.exists(target_path):
                    self._json({"items": [], "current_dir": target})
                    return
                rules = load_exclude_rules()
                items = []
                for item in os.listdir(target_path):
                    full_item = os.path.join(target_path, item)
                    is_dir = os.path.isdir(full_item)
                    rel = os.path.relpath(full_item, base)
                    entry = {
                        "name": item,
                        "is_dir": is_dir,
                        "size": 0,
                        "mtime": 0,
                        "count": None,          # nb d'éléments pour un dossier
                        "path": rel,
                        "ignored": path_is_ignored(rel, rules),
                    }
                    try:
                        st = os.stat(full_item)
                        entry["mtime"] = int(st.st_mtime * 1000)
                        if not is_dir:
                            entry["size"] = st.st_size
                        else:
                            try:
                                entry["count"] = len(os.listdir(full_item))
                            except OSError:
                                entry["count"] = None
                    except OSError:
                        # lien cassé / permission refusée : on garde l'entrée sans métadonnées
                        pass
                    items.append(entry)
                items.sort(key=lambda x: (not x["is_dir"], x["name"].lower()))
                self._json({"items": items, "current_dir": target})
            except Exception as e:
                self._json({"error": str(e)})
        elif p == "/api/search":
            # Recherche récursive bornée dans le sous-arbre du dossier courant.
            qs = parse_qs(urlparse(self.path).query)
            target = qs.get("dir", [""])[0]
            query = qs.get("q", [""])[0].strip().lower()
            base = os.path.abspath(config.GD_DIR)
            root = os.path.abspath(os.path.join(base, target))
            if not root.startswith(base):
                self._json({"error": "Invalid path"})
                return
            if len(query) < 2:
                self._json({"items": [], "query": query})
                return
            MAX_RESULTS, MAX_SCAN = 400, 60000
            rules = load_exclude_rules()
            results, scanned, truncated = [], 0, False
            try:
                for dirpath, dirnames, filenames in os.walk(root):
                    for name, is_dir in ([(d, True) for d in dirnames]
                                         + [(f, False) for f in filenames]):
                        scanned += 1
                        if query in name.lower():
                            full = os.path.join(dirpath, name)
                            rel = os.path.relpath(full, base)
                            entry = {
                                "name": os.path.relpath(full, root).replace(os.sep, "/"),
                                "is_dir": is_dir, "size": 0, "mtime": 0, "count": None,
                                "path": rel, "ignored": path_is_ignored(rel, rules),
                            }
                            try:
                                st = os.stat(full)
                                entry["mtime"] = int(st.st_mtime * 1000)
                                if not is_dir:
                                    entry["size"] = st.st_size
                            except OSError:
                                pass
                            results.append(entry)
                            if len(results) >= MAX_RESULTS:
                                truncated = True
                                break
                    if truncated or scanned > MAX_SCAN:
                        truncated = truncated or scanned > MAX_SCAN
                        break
                results.sort(key=lambda x: (not x["is_dir"], x["name"].lower()))
                self._json({"items": results, "query": query, "truncated": truncated})
            except Exception as e:
                self._json({"error": str(e)})
        elif p == "/api/filters":
            try:
                with open(os.path.expanduser("~/.config/rclone/gdrive-filters.txt"), "r") as f:
                    self._json({"content": f.read()})
            except Exception as e:
                self._json({"error": str(e)})
        elif p == "/api/filters_add":
            qs = parse_qs(urlparse(self.path).query)
            rule = qs.get("rule", [""])[0]
            if rule:
                try:
                    # N'ajoute un saut de ligne que si le fichier n'en finit pas
                    # déjà par un, pour éviter toute ligne vide parasite.
                    prefix = ""
                    try:
                        with open(config.FILTERS_PATH, "r") as f:
                            existing = f.read()
                        if existing and not existing.endswith("\n"):
                            prefix = "\n"
                    except OSError:
                        existing = ""
                    with open(config.FILTERS_PATH, "a") as f:
                        f.write(f"{prefix}{rule}\n")
                    self._json({"ok": True})
                except Exception as e:
                    self._json({"error": str(e)})
            else:
                self._json({"error": "No rule provided"})
        elif p == "/api/open":
            qs = parse_qs(urlparse(self.path).query)
            target = qs.get("path", [""])[0]
            dir_only = qs.get("dir_only", [""])[0] == "1"
            base = os.path.abspath(os.path.expanduser("~/GoogleDrive"))
            target_path = os.path.abspath(os.path.join(base, target))
            
            if dir_only:
                target_path = os.path.dirname(target_path)
                
            if target_path.startswith(base) and os.path.exists(target_path):
                # use xdg-open for linux
                subprocess.Popen(["xdg-open", target_path])
                self._json({"ok": True})
            else:
                self._json({"ok": False})
        elif p == "/api/delete_preview":
            # Prévisualise ce qu'une suppression locale effacerait : liste + taille + statut exclu.
            qs = parse_qs(urlparse(self.path).query)
            tp = self._safe_local(qs.get("path", [""])[0])
            if not tp:
                self._json({"ok": False, "error": "Chemin invalide"})
                return
            base = os.path.abspath(config.GD_DIR)
            rel = os.path.relpath(tp, base)
            files, total, count, truncated = [], 0, 0, False
            if os.path.isdir(tp) and not os.path.islink(tp):
                for root, _dirs, fnames in os.walk(tp):
                    for fn in fnames:
                        fpath = os.path.join(root, fn)
                        try:
                            sz = os.path.getsize(fpath)
                        except OSError:
                            sz = 0
                        total += sz
                        count += 1
                        if len(files) < 300:
                            files.append({"path": os.path.relpath(fpath, tp), "size": sz})
                        else:
                            truncated = True
                is_dir = True
            else:
                try:
                    total = os.path.getsize(tp)
                except OSError:
                    total = 0
                count = 1
                files.append({"path": os.path.basename(tp), "size": total})
                is_dir = False
            files.sort(key=lambda x: -x["size"])
            self._json({
                "ok": True, "is_dir": is_dir, "count": count, "size": total,
                "files": files, "truncated": truncated, "path": rel,
                "ignored": path_is_ignored(rel, load_exclude_rules()),
            })
        elif p == "/api/drive_check":
            # Diff complet local ↔ Drive via « rclone check --combined ».
            qs = parse_qs(urlparse(self.path).query)
            tp = self._safe_local(qs.get("path", [""])[0])
            if not tp:
                self._json({"ok": False, "error": "Chemin invalide"})
                return
            base = os.path.abspath(config.GD_DIR)
            rel = os.path.relpath(tp, base).replace(os.sep, "/")
            try:
                if os.path.isdir(tp) and not os.path.islink(tp):
                    cmd = ["rclone", "check", tp, "GoogleDrive:" + rel, "--combined", "-"]
                else:
                    parent = os.path.dirname(tp)
                    rel_parent = os.path.dirname(rel)
                    remote = "GoogleDrive:" + rel_parent if rel_parent else "GoogleDrive:"
                    cmd = ["rclone", "check", parent, remote,
                           "--combined", "-", "--include", "/" + os.path.basename(tp)]
                proc = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
                res = {"identical": [], "differ": [], "local_only": [], "drive_only": [], "error": []}
                flag_map = {"=": "identical", "*": "differ", "+": "local_only",
                            "-": "drive_only", "!": "error"}
                for line in proc.stdout.splitlines():
                    if len(line) >= 3 and line[1] == " " and line[0] in flag_map:
                        res[flag_map[line[0]]].append(line[2:])
                # « présent sur le Drive » = au moins un fichier vu côté Drive
                # (identique, différent, ou présent uniquement côté Drive).
                if not any(res[k] for k in ("identical", "differ", "drive_only")):
                    self._json({"ok": True, "exists": False})
                    return
                # « intégralement sauvegardé » : rien qui ne soit uniquement local, différent ou en erreur
                fully_backed = not (res["local_only"] or res["differ"] or res["error"])
                self._json({
                    "ok": True, "exists": True, "fully_backed": fully_backed,
                    "counts": {k: len(v) for k, v in res.items()},
                    "result": {k: v[:200] for k, v in res.items()},
                })
            except subprocess.TimeoutExpired:
                self._json({"ok": False, "error": "rclone check : délai dépassé (dossier trop volumineux ?)"})
            except FileNotFoundError:
                self._json({"ok": False, "error": "rclone introuvable"})
            except Exception as e:
                self._json({"ok": False, "error": str(e)})
        elif p == "/api/filters_remove":
            # Retire une règle exacte du fichier de filtres (ré-inclusion).
            qs = parse_qs(urlparse(self.path).query)
            rule = qs.get("rule", [""])[0].strip()
            if not rule:
                self._json({"ok": False, "error": "No rule provided"})
            else:
                try:
                    with open(config.FILTERS_PATH, "r") as f:
                        lines = f.readlines()
                    kept, removed = [], 0
                    for ln in lines:
                        if ln.strip() == rule:
                            removed += 1
                        else:
                            kept.append(ln)
                    if removed:
                        with open(config.FILTERS_PATH, "w") as f:
                            f.writelines(kept)
                    self._json({"ok": True, "removed": removed})
                except Exception as e:
                    self._json({"ok": False, "error": str(e)})
        elif p == "/api/match_rules":
            # Quels motifs d'exclusion s'appliquent à ce chemin ?
            qs = parse_qs(urlparse(self.path).query)
            target = qs.get("path", [""])[0]
            base = os.path.abspath(config.GD_DIR)
            tp = os.path.abspath(os.path.join(base, target))
            if not tp.startswith(base):
                self._json({"ok": False, "error": "Chemin invalide"})
                return
            rel = os.path.relpath(tp, base)
            matched = [pat for pat in load_exclude_rules() if path_is_ignored(rel, [pat])]
            self._json({"ok": True, "rules": matched})
        elif p == "/api/rule_impact":
            # Tout ce qu'une règle d'exclusion affecterait dans l'arbre synchronisé.
            qs = parse_qs(urlparse(self.path).query)
            raw = qs.get("rule", [""])[0].strip()
            pat = raw[2:].strip() if raw[:2] in ("- ", "+ ") else raw
            if not pat or raw.startswith("#") or raw.startswith("+ "):
                self._json({"ok": True, "items": [], "count": 0, "size": 0,
                            "truncated": False, "not_exclusion": True})
                return
            base = os.path.abspath(config.GD_DIR)
            MAX_RESULTS, MAX_SCAN = 500, 100000
            items, total_size, total_count, scanned, truncated = [], 0, 0, 0, False
            try:
                for dirpath, dirnames, filenames in os.walk(base):
                    keep = []
                    for dname in dirnames:
                        scanned += 1
                        full = os.path.join(dirpath, dname)
                        rel = os.path.relpath(full, base).replace(os.sep, "/")
                        if path_is_ignored(rel, [pat]):
                            # dossier exclu : on le rapporte et on ne descend pas dedans
                            dsize, dfiles = 0, 0
                            for dp, _sd, fs in os.walk(full):
                                for f in fs:
                                    try:
                                        dsize += os.path.getsize(os.path.join(dp, f))
                                    except OSError:
                                        pass
                                    dfiles += 1
                            total_size += dsize
                            total_count += 1
                            if len(items) < MAX_RESULTS:
                                items.append({"path": rel, "is_dir": True, "size": dsize, "count": dfiles})
                            else:
                                truncated = True
                        else:
                            keep.append(dname)
                    dirnames[:] = keep
                    for fname in filenames:
                        scanned += 1
                        full = os.path.join(dirpath, fname)
                        rel = os.path.relpath(full, base).replace(os.sep, "/")
                        if path_is_ignored(rel, [pat]):
                            try:
                                fsize = os.path.getsize(full)
                            except OSError:
                                fsize = 0
                            total_size += fsize
                            total_count += 1
                            if len(items) < MAX_RESULTS:
                                items.append({"path": rel, "is_dir": False, "size": fsize})
                            else:
                                truncated = True
                    if scanned > MAX_SCAN:
                        truncated = True
                        break
                items.sort(key=lambda x: (not x["is_dir"], x["path"].lower()))
                self._json({"ok": True, "pattern": pat, "items": items,
                            "count": total_count, "size": total_size, "truncated": truncated})
            except Exception as e:
                self._json({"ok": False, "error": str(e)})
        elif p in ["/", "/index.html"]:
            self._file(
                os.path.join(self._d, "index.html"), "text/html;charset=utf-8"
            )
        elif p == "/style.css":
            self._file(
                os.path.join(self._d, "style.css"), "text/css;charset=utf-8"
            )
        elif p == "/app.js":
            self._file(
                os.path.join(self._d, "app.js"), "application/javascript;charset=utf-8"
            )
        elif p.startswith("/js/") and p.endswith(".js"):
            safe = os.path.normpath(p.lstrip("/"))
            fpath = os.path.join(self._d, safe)
            if os.path.isfile(fpath) and os.path.commonpath([self._d, fpath]) == self._d:
                self._file(fpath, "application/javascript;charset=utf-8")
            else:
                self.send_response(404)
                self.end_headers()
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        p = urlparse(self.path).path
        if p == "/api/filters_save":
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                post_data = self.rfile.read(content_length).decode('utf-8')
                data = json.loads(post_data)
                content = data.get("content", "")
                with open(os.path.expanduser("~/.config/rclone/gdrive-filters.txt"), "w") as f:
                    f.write(content)
                self._json({"ok": True})
            except Exception as e:
                self._json({"error": str(e)})
        elif p == "/api/delete":
            # Suppression locale (irréversible) — garde-fous : dans la base, jamais la base elle-même.
            try:
                length = int(self.headers.get("Content-Length", 0))
                data = json.loads(self.rfile.read(length).decode("utf-8")) if length else {}
                tp = self._safe_local(data.get("path", ""))
                if not tp:
                    self._json({"ok": False, "error": "Chemin invalide"})
                    return
                freed = 0
                if os.path.isdir(tp) and not os.path.islink(tp):
                    for root, _dirs, fnames in os.walk(tp):
                        for fn in fnames:
                            try:
                                freed += os.path.getsize(os.path.join(root, fn))
                            except OSError:
                                pass
                    shutil.rmtree(tp)
                else:
                    try:
                        freed = os.path.getsize(tp)
                    except OSError:
                        freed = 0
                    os.remove(tp)
                self._json({"ok": True, "freed": freed})
            except Exception as e:
                self._json({"ok": False, "error": str(e)})
        elif p == "/api/drive_delete":
            # Suppression côté Google Drive (irréversible) — purge (dossier) ou deletefile (fichier).
            try:
                length = int(self.headers.get("Content-Length", 0))
                data = json.loads(self.rfile.read(length).decode("utf-8")) if length else {}
                rel = self._safe_rel(data.get("path", ""))
                if not rel:
                    self._json({"ok": False, "error": "Chemin invalide"})
                    return
                is_dir = bool(data.get("is_dir"))
                remote = "GoogleDrive:" + rel
                cmd = ["rclone", "purge", remote] if is_dir else ["rclone", "deletefile", remote]
                proc = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
                if proc.returncode == 0:
                    self._json({"ok": True})
                else:
                    self._json({"ok": False, "error": (proc.stderr or "échec rclone").strip()[-400:]})
            except subprocess.TimeoutExpired:
                self._json({"ok": False, "error": "rclone : délai dépassé"})
            except FileNotFoundError:
                self._json({"ok": False, "error": "rclone introuvable"})
            except Exception as e:
                self._json({"ok": False, "error": str(e)})
        elif p == "/api/rule_delete":
            # Supprime TOUT ce qu'une règle exclut, en local et/ou sur le Drive.
            try:
                length = int(self.headers.get("Content-Length", 0))
                data = json.loads(self.rfile.read(length).decode("utf-8")) if length else {}
                raw = (data.get("rule") or "").strip()
                mode = data.get("mode") or "none"
                pat = raw[2:].strip() if raw[:2] in ("- ", "+ ") else raw
                if not pat or mode not in ("local", "drive", "both"):
                    self._json({"ok": False, "error": "Requête invalide"})
                    return
                base = os.path.abspath(config.GD_DIR)
                MAX_TARGETS, MAX_SCAN = 2000, 300000
                targets, scanned, truncated = [], 0, False
                for dirpath, dirnames, filenames in os.walk(base):
                    keep = []
                    for dname in dirnames:
                        scanned += 1
                        rel = os.path.relpath(os.path.join(dirpath, dname), base).replace(os.sep, "/")
                        if path_is_ignored(rel, [pat]):
                            targets.append((rel, True))   # dossier exclu : on l'élague
                        else:
                            keep.append(dname)
                    dirnames[:] = keep
                    for fname in filenames:
                        scanned += 1
                        rel = os.path.relpath(os.path.join(dirpath, fname), base).replace(os.sep, "/")
                        if path_is_ignored(rel, [pat]):
                            targets.append((rel, False))
                    if len(targets) >= MAX_TARGETS or scanned > MAX_SCAN:
                        truncated = True
                        break
                freed, ndel, errors = 0, 0, []
                for rel, is_dir in targets:
                    full = os.path.abspath(os.path.join(base, rel))
                    if full == base or not full.startswith(base + os.sep):
                        continue
                    if mode in ("local", "both") and os.path.exists(full):
                        try:
                            if os.path.isdir(full) and not os.path.islink(full):
                                for dp, _sd, fs in os.walk(full):
                                    for f in fs:
                                        try:
                                            freed += os.path.getsize(os.path.join(dp, f))
                                        except OSError:
                                            pass
                                shutil.rmtree(full)
                            else:
                                try:
                                    freed += os.path.getsize(full)
                                except OSError:
                                    pass
                                os.remove(full)
                        except Exception as e:
                            errors.append(f"local {rel} : {e}")
                    if mode in ("drive", "both"):
                        cmd = (["rclone", "purge", "GoogleDrive:" + rel] if is_dir
                               else ["rclone", "deletefile", "GoogleDrive:" + rel])
                        try:
                            pr = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
                            if pr.returncode != 0:
                                errors.append(f"drive {rel} : " + (pr.stderr or "").strip()[-120:])
                        except Exception as e:
                            errors.append(f"drive {rel} : {e}")
                    ndel += 1
                self._json({"ok": True, "count": ndel, "freed": freed,
                            "truncated": truncated, "errors": errors[:20]})
            except Exception as e:
                self._json({"ok": False, "error": str(e)})
        else:
            self.send_response(404)
            self.end_headers()

    def _safe_local(self, target):
        """Résout un chemin relatif sous la base synchronisée.

        Renvoie le chemin absolu s'il est valide (dans la base, différent de la
        base, et existant), sinon None. Bloque toute traversée hors du dossier.
        """
        base = os.path.abspath(config.GD_DIR)
        tp = os.path.abspath(os.path.join(base, target or ""))
        if tp == base or not tp.startswith(base + os.sep) or not os.path.exists(tp):
            return None
        return tp

    def _safe_rel(self, target):
        """Valide un chemin relatif (dans la base, ≠ base, sans traversée) et le
        renvoie normalisé — sans exiger qu'il existe localement (utile pour le Drive)."""
        base = os.path.abspath(config.GD_DIR)
        tp = os.path.abspath(os.path.join(base, target or ""))
        if tp == base or not tp.startswith(base + os.sep):
            return None
        return os.path.relpath(tp, base).replace(os.sep, "/")

    def _json(self, d):
        b = json.dumps(d).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", len(b))
        self.end_headers()
        self.wfile.write(b)

    def _file(self, path, mime):
        try:
            with open(path, "rb") as f:
                b = f.read()
            self.send_response(200)
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", len(b))
            self.end_headers()
            self.wfile.write(b)
        except Exception:
            self.send_response(404)
            self.end_headers()

