import os
import subprocess
import shutil
import re
import glob
from typing import Dict, Any

from . import config
from .filters import load_exclude_rules, path_is_ignored

def _within_base(target: str):
    base = os.path.abspath(config.GD_DIR)
    tp = os.path.abspath(os.path.join(base, target or ""))
    if tp != base and not tp.startswith(base + os.sep):
        return None
    return tp

def _safe_local(target: str):
    base = os.path.abspath(config.GD_DIR)
    tp = os.path.abspath(os.path.join(base, target or ""))
    if tp == base or not tp.startswith(base + os.sep) or not os.path.exists(tp):
        return None
    return tp

def _safe_rel(target: str):
    base = os.path.abspath(config.GD_DIR)
    tp = os.path.abspath(os.path.join(base, target or ""))
    if tp == base or not tp.startswith(base + os.sep):
        return None
    return os.path.relpath(tp, base).replace(os.sep, "/")

def api_trigger(m):
    ok, err = m.trigger()
    return {"ok": ok, "error": err}


def api_cancel(m):
    ok, err = m.cancel()
    return {"ok": ok, "error": err}


def api_bwlimit():
    try:
        limit = ""
        if os.path.exists(config.BWLIMIT_PATH):
            with open(config.BWLIMIT_PATH, "r") as f:
                match = re.search(r"RCLONE_BWLIMIT=(.*)", f.read())
                if match:
                    limit = match.group(1).strip()
        return {"limit": limit}
    except Exception as e:
        return {"error": str(e)}


def api_bwlimit_save(limit: str = ""):
    try:
        os.makedirs(os.path.dirname(config.BWLIMIT_PATH), exist_ok=True)
        with open(config.BWLIMIT_PATH, "w") as f:
            f.write(f"RCLONE_BWLIMIT={limit}\n" if limit else "")
        return {"ok": True}
    except Exception as e:
        return {"error": str(e)}


def api_dryrun():
    try:
        cache_dir = os.path.expanduser("~/.cache/rclone/bisync")
        if os.path.exists(cache_dir):
            for f in glob.glob(os.path.join(cache_dir, "*.lst")):
                try:
                    shutil.copy2(f, f + "-dry")
                except Exception:
                    pass
        cmd = ["rclone", "bisync", config.REMOTE, config.GD_DIR, "--dry-run", "-v", "--tpslimit", "8", "--filter-from", config.FILTERS_PATH]
        out = subprocess.check_output(cmd, stderr=subprocess.STDOUT, timeout=600).decode('utf-8', errors='ignore')
        out = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', out)
        return {"ok": True, "log": out}
    except subprocess.CalledProcessError as e:
        out = e.output.decode('utf-8', errors='ignore')
        out = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', out)
        return {"ok": False, "error": out}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def api_tree(dir: str = ""):
    target = dir
    base = os.path.abspath(config.GD_DIR)
    target_path = _within_base(target)
    if target_path is None:
        return {"error": "Invalid path"}
    try:
        if not os.path.exists(target_path):
            return {"items": [], "current_dir": target}
        rules = load_exclude_rules()
        items = []
        for item in os.listdir(target_path):
            full_item = os.path.join(target_path, item)
            is_dir = os.path.isdir(full_item)
            rel = os.path.relpath(full_item, base)
            entry = {
                "name": item, "is_dir": is_dir, "size": 0, "mtime": 0,
                "count": None, "path": rel, "ignored": path_is_ignored(rel, rules)
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
                pass
            items.append(entry)
        items.sort(key=lambda x: (not x["is_dir"], x["name"].lower()))
        return {"items": items, "current_dir": target}
    except Exception as e:
        return {"error": str(e)}


def api_search(dir: str = "", q: str = ""):
    target = dir
    query = q.strip().lower()
    base = os.path.abspath(config.GD_DIR)
    root = _within_base(target)
    if root is None:
        return {"error": "Invalid path"}
    if len(query) < 2:
        return {"items": [], "query": query}
    MAX_RESULTS, MAX_SCAN = 400, 60000
    rules = load_exclude_rules()
    results, scanned, truncated = [], 0, False
    try:
        for dirpath, dirnames, filenames in os.walk(root):
            for name, is_dir in ([(d, True) for d in dirnames] + [(f, False) for f in filenames]):
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
        return {"items": results, "query": query, "truncated": truncated}
    except Exception as e:
        return {"error": str(e)}


def api_filters():
    try:
        with open(config.FILTERS_PATH, "r") as f:
            return {"content": f.read()}
    except Exception as e:
        return {"error": str(e)}


def api_filters_add(rule: str = ""):
    if not rule:
        return {"error": "No rule provided"}
    try:
        prefix = ""
        try:
            with open(config.FILTERS_PATH, "r") as f:
                existing = f.read()
            if existing and not existing.endswith("\n"):
                prefix = "\n"
        except OSError:
            pass
        with open(config.FILTERS_PATH, "a") as f:
            f.write(f"{prefix}{rule}\n")
        return {"ok": True}
    except Exception as e:
        return {"error": str(e)}


def api_open(path: str = "", dir_only: str = "0"):
    target = path
    dir_only_bool = dir_only == "1"
    base = os.path.abspath(config.GD_DIR)
    target_path = _within_base(target)
    if target_path is not None and dir_only_bool:
        target_path = None if target_path == base else os.path.dirname(target_path)
    if target_path is not None and os.path.exists(target_path):
        subprocess.Popen(["xdg-open", target_path])
        return {"ok": True}
    return {"ok": False}


def api_delete_preview(path: str = ""):
    tp = _safe_local(path)
    if not tp:
        return {"ok": False, "error": "Chemin invalide"}
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
    return {
        "ok": True, "is_dir": is_dir, "count": count, "size": total,
        "files": files, "truncated": truncated, "path": rel,
        "ignored": path_is_ignored(rel, load_exclude_rules())
    }


def api_drive_check(path: str = ""):
    tp = _safe_local(path)
    if not tp:
        return {"ok": False, "error": "Chemin invalide"}
    base = os.path.abspath(config.GD_DIR)
    rel = os.path.relpath(tp, base).replace(os.sep, "/")
    try:
        if os.path.isdir(tp) and not os.path.islink(tp):
            cmd = ["rclone", "check", tp, config.REMOTE + rel, "--combined", "-"]
        else:
            parent = os.path.dirname(tp)
            rel_parent = os.path.dirname(rel)
            remote = config.REMOTE + rel_parent if rel_parent else config.REMOTE
            cmd = ["rclone", "check", parent, remote, "--combined", "-", "--include", "/" + os.path.basename(tp)]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
        res = {"identical": [], "differ": [], "local_only": [], "drive_only": [], "error": []}
        flag_map = {"=": "identical", "*": "differ", "+": "local_only", "-": "drive_only", "!": "error"}
        for line in proc.stdout.splitlines():
            if len(line) >= 3 and line[1] == " " and line[0] in flag_map:
                res[flag_map[line[0]]].append(line[2:])
        if not any(res[k] for k in ("identical", "differ", "drive_only")):
            return {"ok": True, "exists": False}
        fully_backed = not (res["local_only"] or res["differ"] or res["error"])
        return {
            "ok": True, "exists": True, "fully_backed": fully_backed,
            "counts": {k: len(v) for k, v in res.items()},
            "result": {k: v[:200] for k, v in res.items()}
        }
    except Exception as e:
        return {"ok": False, "error": str(e)}


def api_filters_remove(rule: str = ""):
    rule = rule.strip()
    if not rule:
        return {"ok": False, "error": "No rule provided"}
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
        return {"ok": True, "removed": removed}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def api_match_rules(path: str = ""):
    base = os.path.abspath(config.GD_DIR)
    tp = _within_base(path)
    if tp is None:
        return {"ok": False, "error": "Chemin invalide"}
    rel = os.path.relpath(tp, base)
    matched = [pat for pat in load_exclude_rules() if path_is_ignored(rel, [pat])]
    return {"ok": True, "rules": matched}


def api_rule_impact(rule: str = ""):
    raw = rule.strip()
    pat = raw[2:].strip() if raw[:2] in ("- ", "+ ") else raw
    if not pat or raw.startswith("#") or raw.startswith("+ "):
        return {"ok": True, "items": [], "count": 0, "size": 0, "truncated": False, "not_exclusion": True}
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
        return {"ok": True, "pattern": pat, "items": items, "count": total_count, "size": total_size, "truncated": truncated}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def api_filters_save(data: Dict[str, Any]):
    try:
        content = data.get("content", "")
        with open(config.FILTERS_PATH, "w") as f:
            f.write(content)
        return {"ok": True}
    except Exception as e:
        return {"error": str(e)}


def api_delete(data: Dict[str, Any]):
    try:
        tp = _safe_local(data.get("path", ""))
        if not tp:
            return {"ok": False, "error": "Chemin invalide"}
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
        return {"ok": True, "freed": freed}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def api_drive_delete(data: Dict[str, Any]):
    try:
        rel = _safe_rel(data.get("path", ""))
        if not rel:
            return {"ok": False, "error": "Chemin invalide"}
        is_dir = bool(data.get("is_dir"))
        remote = config.REMOTE + rel
        cmd = ["rclone", "purge", remote] if is_dir else ["rclone", "deletefile", remote]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
        if proc.returncode == 0:
            return {"ok": True}
        else:
            return {"ok": False, "error": (proc.stderr or "échec rclone").strip()[-400:]}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def api_rule_delete(data: Dict[str, Any]):
    try:
        raw = (data.get("rule") or "").strip()
        mode = data.get("mode") or "none"
        pat = raw[2:].strip() if raw[:2] in ("- ", "+ ") else raw
        if not pat or mode not in ("local", "drive", "both"):
            return {"ok": False, "error": "Requête invalide"}
        base = os.path.abspath(config.GD_DIR)
        MAX_TARGETS, MAX_SCAN = 2000, 300000
        targets, scanned, truncated = [], 0, False
        for dirpath, dirnames, filenames in os.walk(base):
            keep = []
            for dname in dirnames:
                scanned += 1
                rel = os.path.relpath(os.path.join(dirpath, dname), base).replace(os.sep, "/")
                if path_is_ignored(rel, [pat]):
                    targets.append((rel, True))
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
                cmd = ["rclone", "purge", config.REMOTE + rel] if is_dir else ["rclone", "deletefile", config.REMOTE + rel]
                try:
                    pr = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
                    if pr.returncode != 0:
                        errors.append(f"drive {rel} : " + (pr.stderr or "").strip()[-120:])
                except Exception as e:
                    errors.append(f"drive {rel} : {e}")
            ndel += 1
        return {"ok": True, "count": ndel, "freed": freed, "truncated": truncated, "errors": errors[:20]}
    except Exception as e:
        return {"ok": False, "error": str(e)}


