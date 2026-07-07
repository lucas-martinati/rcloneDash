#!/usr/bin/env python3
"""
rclone-monitor.py — Serveur local amélioré pour surveiller rclone-bisync
Usage : python3 rclone-monitor.py
Puis   : http://localhost:8765

Fonctionnalités :
  - Streaming live des logs via journalctl -f (thread dédié)
  - Parsing intelligent des phases de sync, stats de transfert, fichier actif
  - KPIs enrichis : fichiers GDrive, conflits, taux de succès 7j, vitesse
  - Historique détaillé avec copied/modified/deleted/elapsed par run
  - Fichiers récents (20 derniers touchés)
  - Alertes : échecs consécutifs, sync lente, disque critique

Contraintes : Python 3 stdlib uniquement, aucune lib externe.
"""

import http.server
import socketserver
import json
import subprocess
import threading
import os
import shutil
import re
import time
from collections import deque
from datetime import datetime
from urllib.parse import urlparse, parse_qs

PORT = 8765
GD_DIR = os.path.expanduser("~/GoogleDrive")


def local_size(fpath):
    """Taille du fichier local (en octets), ou None si introuvable."""
    try:
        return os.path.getsize(os.path.join(GD_DIR, fpath))
    except OSError:
        return None


FILTERS_PATH = os.path.expanduser("~/.config/rclone/gdrive-filters.txt")


def load_exclude_rules():
    """Renvoie la liste des motifs d'exclusion (lignes « - motif ») du fichier de filtres."""
    rules = []
    try:
        with open(FILTERS_PATH, "r") as f:
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
        if pat[i:i + 3] == "**/":
            out.append("(?:.*/)?")
            i += 3
        elif pat[i:i + 2] == "**":
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

# Phases d'une sync bisync, dans l'ordre
PHASES = [
    "Building listings",
    "Path1 checking for diffs",
    "Path2 checking for diffs",
    "Applying changes",
    "Updating listings",
    "Bisync successful",
]


class LogStreamer(threading.Thread):
    """Thread daemon qui streame journalctl -f et parse les données de sync live."""

    def __init__(self, service):
        super().__init__(daemon=True)
        self.service = service
        self.lock = threading.Lock()
        self.buffer = deque(maxlen=500)
        self._reset()

    def _reset(self):
        self.phase = ""
        self.phase_index = -1
        self.transfer = {}
        self.active_file = ""
        self.active_file_pct = 0
        self.active_files = {}
        self.synced_files = []
        self.changes = {
            "path1": {"new": [], "modified": [], "deleted": []},
            "path2": {"new": [], "modified": [], "deleted": []},
        }
        self.is_syncing = False
        self.sync_start = 0

    def run(self):
        """Boucle infinie : lance journalctl -f, re-lance si ça plante."""
        while True:
            try:
                proc = subprocess.Popen(
                    [
                        "journalctl", "-f", "-u", self.service,
                        "--output=short-iso", "-n", "0",
                    ],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                for line in iter(proc.stdout.readline, ""):
                    line = line.strip()
                    if not line:
                        continue
                    with self.lock:
                        self.buffer.append(line)
                        self._parse(line)
            except Exception:
                time.sleep(5)

    def _parse(self, line):
        ll = line.lower()

        # Détection du démarrage d'une sync : uniquement la ligne systemd,
        # sinon un log rclone comme "Starting transaction limiter" réinitialise
        # la progression en plein milieu d'une sync.
        if "systemd" in ll and ("starting" in ll or "started" in ll) \
                and self.service + ".service" in ll:
            self._reset()
            self.is_syncing = True
            self.phase = "Building listings"
            self.phase_index = 0
            self.sync_start = time.time()
            return

        if not self.is_syncing:
            return

        # Détection de la phase courante
        for i, ph in enumerate(PHASES):
            if ph.lower() in ll:
                self.phase = ph
                self.phase_index = i
                break

        # Stats de transfert : Transferred: 1.234 MiB / 5.678 MiB, 22%, ...
        m = re.search(
            r"transferred:\s+([\d.]+\s*\S+)\s*/\s*([\d.]+\s*\S+),\s*(\d+)%", ll
        )
        if m:
            self.transfer["done"] = m.group(1)
            self.transfer["total"] = m.group(2)
            self.transfer["pct"] = int(m.group(3))
            sm = re.search(r"([\d.]+\s*\S+/s)", line)
            if sm:
                self.transfer["speed"] = sm.group(1)
            em = re.search(r"ETA\s+(\S+)", line)
            if em:
                self.transfer["eta"] = em.group(1)

        # Compteur de fichiers : Transferred: 5 / 10, 50%
        m = re.search(r"transferred:\s+(\d+)\s*/\s*(\d+),", ll)
        if m:
            self.transfer["files_done"] = int(m.group(1))
            self.transfer["files_total"] = int(m.group(2))

        # Checks: 45 / 100, 45%
        m = re.search(r"checks:\s+(\d+)\s*/\s*(\d+)", ll)
        if m:
            self.transfer["checks_done"] = int(m.group(1))
            self.transfer["checks_total"] = int(m.group(2))

        # Elapsed time: 3.5s
        m = re.search(r"elapsed time:\s*(\S+)", ll)
        if m:
            self.transfer["elapsed"] = m.group(1)

        # Fichier actif : * path/to/file: 45% /1.234Mi, 4.5Mi/s, 2m3s
        m = re.search(
            r"\*\s+(.+?):\s*(\d+)%\s*/([^,]+),\s*([^,]+),\s*(\S+)", line
        )
        if not m:
            # Forme courte sans vitesse/ETA : * file: 45% /1.234Mi
            m = re.search(r"\*\s+(.+?):\s*(\d+)%\s*/(\S+)", line)
        if m:
            fname = m.group(1).strip()
            pct = int(m.group(2))
            groups = m.groups()
            self.active_files[fname] = {
                "pct": pct,
                "size": groups[2].strip() if len(groups) > 2 else "",
                "speed": groups[3].strip() if len(groups) > 3 else "",
                "eta": groups[4].strip() if len(groups) > 4 else "",
                "last_seen": time.time(),
            }
            self.active_file = fname
            self.active_file_pct = pct

        # Fichier en cours de vérification : * path/to/file: checking
        m = re.search(r"\*\s+(.+?):\s*(checking|transferring)\s*$", line)
        if m:
            fname = m.group(1).strip()
            self.active_files[fname] = {
                "pct": 0,
                "size": "",
                "speed": "",
                "eta": "",
                "status": m.group(2),
                "last_seen": time.time(),
            }

        # Fichiers synchronisés durant le run courant
        m = re.search(r"INFO\s+:\s+(.*?):\s+(Copied \(new\)|Copied \(replaced existing\)|Updated modification time in destination|Deleted|Updated file)", line)
        if m:
            fpath = m.group(1).strip()
            action = m.group(2).strip()
            act = "new"
            if "Deleted" in action:
                act = "deleted"
            elif "Copied (replaced existing)" in action or "Updated modification" in action or "Updated file" in action:
                act = "modified"
                
            if not any(f["path"] == fpath for f in self.synced_files):
                self.synced_files.append({
                    "path": fpath,
                    "action": act,
                    "size": None if act == "deleted" else local_size(fpath),
                    "time": datetime.now().strftime("%H:%M:%S")
                })

        # Changements détectés sur Path1/Path2
        for pk, pl in [("path1", "Path1"), ("path2", "Path2")]:
            if pl in line:
                # File is new
                m = re.search(rf"{pl}\s+File is new\s+-\s+(.+)", line)
                if m:
                    f = m.group(1).strip()
                    if f not in self.changes[pk]["new"]:
                        self.changes[pk]["new"].append(f)
                    continue
                # File changed
                m = re.search(rf"{pl}\s+File changed:.*?\s+-\s+(.+)", line)
                if m:
                    f = m.group(1).strip()
                    if f not in self.changes[pk]["modified"]:
                        self.changes[pk]["modified"].append(f)
                    continue
                # File was deleted
                m = re.search(rf"{pl}\s+File was deleted\s+-\s+(.+)", line)
                if m:
                    f = m.group(1).strip()
                    if f not in self.changes[pk]["deleted"]:
                        self.changes[pk]["deleted"].append(f)
                    continue

        # Fin de sync
        if "bisync successful" in ll:
            self.phase = "Bisync successful"
            self.phase_index = 5
            self.is_syncing = False
        elif "systemd" in ll and "finished" in ll and self.service in ll:
            self.is_syncing = False
        elif "systemd" in ll and "failed" in ll and self.service in ll:
            self.is_syncing = False
            subprocess.run(["notify-send", "RcloneDash", "Échec de la synchronisation. Consultez le tableau de bord.", "--icon=dialog-error", "-u", "critical"], check=False)

    def get_live(self):
        """Retourne l'état live de la sync en cours, ou None."""
        with self.lock:
            if self.phase_index < 0:
                return None
            
            now = time.time()
            self.active_files = {
                k: v for k, v in self.active_files.items()
                if now - v["last_seen"] < 4.0
            }
            
            active_list = [
                {
                    "name": k,
                    "pct": v["pct"],
                    "size": v.get("size", ""),
                    "speed": v.get("speed", ""),
                    "eta": v.get("eta", ""),
                    "status": v.get("status", ""),
                }
                for k, v in self.active_files.items()
            ]
            
            if active_list:
                self.active_file = active_list[0]["name"]
                self.active_file_pct = active_list[0]["pct"]
            else:
                self.active_file = ""
                self.active_file_pct = 0
                
            return {
                "phase": self.phase,
                "phase_index": self.phase_index,
                "phases_total": len(PHASES),
                "transfer": dict(self.transfer),
                "active_file": self.active_file,
                "active_file_pct": self.active_file_pct,
                "active_files": active_list,
                "synced_files": list(self.synced_files),
                "changes": {
                    k: {a: list(v) for a, v in d.items()}
                    for k, d in self.changes.items()
                },
                "is_syncing": self.is_syncing,
                "duration_s": int(time.time() - self.sync_start)
                if self.sync_start
                else 0,
            }


class Monitor:
    """Collecte toutes les données de monitoring rclone-bisync."""

    def __init__(self):
        self.svc = "rclone-bisync"
        self.tmr = "rclone-bisync.timer"
        self.gd = GD_DIR
        self.lock = threading.Lock()
        self.streamer = LogStreamer(self.svc)
        self.streamer.start()

        self.quota = None

        # Caches
        self._parsed = None
        self._parsed_time = 0
        self._rate = 100.0
        self._rate_time = 0
        self._logs = None
        self._logs_time = 0
        self._fcount = 0
        self._fcount_time = 0

    def cmd(self, c, timeout=10):
        """Exécute une commande et retourne (stdout, stderr, returncode)."""
        try:
            r = subprocess.run(c, capture_output=True, text=True, timeout=timeout)
            return r.stdout.strip(), r.stderr.strip(), r.returncode
        except Exception as e:
            return "", str(e), 1

    def timer(self):
        """État du timer systemd."""
        o, _, _ = self.cmd(
            ["systemctl", "status", self.tmr, "--no-pager", "-l"]
        )
        s = {"active": False, "next_run": "—", "last_run": "—"}
        for l in (o or "").split("\n"):
            if "Active:" in l:
                s["active"] = "active" in l.lower() and "inactive" not in l.lower()
            if "Trigger:" in l:
                s["next_run"] = l.strip().replace("Trigger: ", "")
            if "Triggered:" in l:
                s["last_run"] = l.strip().replace("Triggered: ", "")
        return s

    def service(self):
        """État du service systemd."""
        o, _, _ = self.cmd(
            ["systemctl", "status", self.svc, "--no-pager", "-l"]
        )
        s = {"state": "idle", "result": "—", "duration": "—"}
        for l in (o or "").split("\n"):
            if "Active:" in l:
                if "active (running)" in l or "activating" in l:
                    s["state"] = "running"
                elif "failed" in l.lower():
                    s["state"] = "failed"
                else:
                    s["state"] = "idle"
            if "Result:" in l:
                s["result"] = l.strip().split("Result:")[-1].strip()
                if "success" in s["result"]:
                    s["state"] = "success"
            if "Duration:" in l:
                s["duration"] = l.strip().split("Duration:")[-1].strip()
        return s

    def update_quota(self):
        while True:
            try:
                out = subprocess.check_output(
                    ["rclone", "about", "GoogleDrive:", "--json"], 
                    timeout=30,
                    stderr=subprocess.STDOUT
                ).decode('utf-8')
                self.quota = json.loads(out)
            except subprocess.CalledProcessError as e:
                # On garde la dernière valeur valide plutôt que d'afficher
                # une erreur transitoire pendant 5 minutes.
                if not self.quota or "error" in self.quota:
                    self.quota = {"error": str(e.output.decode('utf-8', errors='ignore')).strip()}
            except Exception as e:
                if not self.quota or "error" in self.quota:
                    self.quota = {"error": str(e)}
            time.sleep(300)

    def disk(self):
        """Usage disque du dossier GoogleDrive."""
        try:
            p = self.gd if os.path.exists(self.gd) else os.path.expanduser("~")
            t, u, f = shutil.disk_usage(p)
            g = 1024 ** 3
            return {
                "total": round(t / g, 1),
                "used": round(u / g, 1),
                "free": round(f / g, 1),
                "pct": round(u / t * 100, 1),
            }
        except Exception:
            return {"total": 0, "used": 0, "free": 0, "pct": 0}

    def count_files(self):
        """Nombre de fichiers dans ~/GoogleDrive, avec timeout 3s et cache 60s."""
        now = time.time()
        if now - self._fcount_time < 60:
            return self._fcount
        if not os.path.isdir(self.gd):
            return 0
        count = 0
        start = time.time()
        try:
            for _, _, files in os.walk(self.gd):
                count += len(files)
                if time.time() - start > 3.0:
                    break
        except Exception:
            pass
        self._fcount = count
        self._fcount_time = now
        return count

    def logs(self, n=150):
        """Logs récents pour le panel d'affichage, cachés 5s."""
        now = time.time()
        if now - self._logs_time < 5 and self._logs is not None:
            return self._logs
        o, _, _ = self.cmd(
            ["journalctl", "-u", self.svc, "--no-pager", "-n", str(n),
             "--output=short-iso"]
        )
        rows = []
        for l in (o or "").split("\n"):
            if not l.strip():
                continue
            ll = l.lower()
            lv = "info"
            if any(w in ll for w in ["error", "failed", "fatal", "errno", "corrupt"]):
                lv = "error"
            elif any(w in ll for w in ["warn", "skipped", "conflict"]):
                lv = "warn"
            elif any(w in ll for w in ["copied", "moved", "deleted", "transferred"]):
                lv = "ok"
            rows.append({"t": l, "l": lv})
        self._logs = rows
        self._logs_time = now
        return rows

    def parse_runs(self):
        """Parse les logs récents pour extraire : runs enrichis, fichiers récents, KPIs.
        
        Une seule requête journalctl -n 2000, résultat caché 10s.
        """
        now = time.time()
        if now - self._parsed_time < 10 and self._parsed is not None:
            return self._parsed

        o, _, _ = self.cmd(
            ["journalctl", "-u", self.svc, "--no-pager", "-n", "2000",
             "--output=short-iso"]
        )
        lines = (o or "").split("\n")
        runs = []
        cur = None
        recent_files = []
        conflicts = 0
        last_error = ""
        today = datetime.now().strftime("%Y-%m-%d")
        avg_speed = "—"

        for l in lines:
            if not l.strip():
                continue
            ll = l.lower()
            ts = l.split(" ")[0] if l else ""

            # Détection du début d'un run : uniquement la ligne systemd
            # ("Starting rclone-bisync.service - ..."), sinon les logs rclone
            # du type "Starting transaction limiter" créent des runs fantômes
            # (l'identifiant journal "rclone-bisync-guard.sh" contient déjà
            # "rclone" et "bisync").
            if "systemd" in ll and ("starting" in ll or "started" in ll) \
                    and self.svc + ".service" in ll:
                if cur:
                    runs.append(cur)
                cur = {
                    "start": ts, "end": "", "status": "running",
                    "files": 0, "errors": 0, "error_logs": [],
                    "copied": 0, "modified": 0, "deleted": 0,
                    "elapsed": "",
                    "synced_files": [],
                }
                continue

            if cur:
                # Réveil "à vide" : la garde a décidé de ne PAS lancer le bisync
                # (aucun changement local et sync complet récent). On marque le run
                # pour l'écarter de l'historique — ce n'est pas une vraie sync.
                if "rclonedash" in ll and "ignoré" in ll:
                    cur["skipped"] = True

                # Extraction des fichiers synchronisés (Copied / Updated / Deleted)
                m = re.search(r"INFO\s+:\s+(.*?):\s+(Copied \(new\)|Copied \(replaced existing\)|Updated modification time in destination|Deleted|Updated file)", l)
                if m:
                    fpath = m.group(1).strip()
                    action = m.group(2).strip()
                    
                    rf = None
                    if "Copied (new)" in action:
                        cur["copied"] += 1
                        cur["files"] += 1
                        rf = {"action": "new", "path": fpath, "time": ts, "size": local_size(fpath)}
                    elif "Copied (replaced existing)" in action or "Updated modification" in action or "Updated file" in action:
                        cur["modified"] += 1
                        cur["files"] += 1
                        rf = {"action": "modified", "path": fpath, "time": ts, "size": local_size(fpath)}
                    elif "Deleted" in action:
                        cur["deleted"] += 1
                        cur["files"] += 1
                        rf = {"action": "deleted", "path": fpath, "time": ts, "size": None}
                    
                    if rf:
                        recent_files.append(rf)
                        cur["synced_files"].append(rf)

                # Elapsed time
                em = re.search(r"elapsed time:\s*(\S+)", ll)
                if em:
                    cur["elapsed"] = em.group(1)

                # Vitesse de transfert
                sm = re.search(r"([\d.]+\s*\S+/s)", l)
                if sm and "transferred" in ll:
                    avg_speed = sm.group(1)

                # Erreurs
                if (
                    any(w in ll for w in ["error", "errno", "fatal"])
                    and not any(w in ll for w in ["starting", "started"])
                ):
                    cur["errors"] += 1
                    last_error = l.strip()
                    cur["error_logs"].append(l.strip())

                # Fin de run : Succeeded (ligne systemd uniquement)
                if "systemd" in ll and "finished" in ll and self.svc in ll:
                    if cur.get("skipped"):
                        cur = None
                        continue
                    cur["status"] = "success"
                    cur["end"] = ts
                    runs.append(cur)
                    cur = None
                    continue

                # Fin de run : Failed (ligne systemd uniquement, sinon un log
                # rclone contenant "failed" clôturerait le run prématurément)
                if "systemd" in ll and "failed" in ll and self.svc in ll:
                    if cur.get("skipped"):
                        cur = None
                        continue
                    cur["status"] = "failed"
                    cur["end"] = ts
                    if not last_error:
                        last_error = l.strip()
                    runs.append(cur)
                    cur = None
                    continue

            # Conflits aujourd'hui
            if "conflict" in ll and ts.startswith(today):
                conflicts += 1

        if cur and not cur.get("skipped"):
            runs.append(cur)

        # Échecs consécutifs (plus récent en premier)
        rev = list(reversed(runs[-15:]))
        consec = 0
        for r in rev:
            if r["status"] == "failed":
                consec += 1
            elif r["status"] == "success":
                break

        self._parsed = {
            "runs": rev,
            "recent_files": recent_files[-100:],
            "avg_speed": avg_speed,
            "conflicts_today": conflicts,
            "consecutive_failures": consec,
            "last_error_msg": last_error if consec >= 2 else "",
        }
        self._parsed_time = now
        return self._parsed

    def success_rate_7d(self):
        """Taux de succès sur 7 jours, caché 120s."""
        now = time.time()
        if now - self._rate_time < 120:
            return self._rate
        o, _, _ = self.cmd(
            ["journalctl", "-u", self.svc, "--no-pager",
             "--since", "7 days ago", "--output=short-iso", "-n", "10000"],
            timeout=15,
        )
        total = 0
        ok = 0
        for l in (o or "").split("\n"):
            ll = l.lower()
            if "starting" in ll and "rclone-bisync.service" in ll:
                total += 1
            if "finished" in ll and "rclone-bisync.service" in ll:
                ok += 1
        self._rate = round(ok / total * 100, 1) if total > 0 else 100.0
        self._rate_time = now
        return self._rate

    def trigger(self):
        """Lance une sync manuelle (--no-block pour ne pas attendre la fin)."""
        # On pose un marqueur "force" pour que la garde lance le bisync sans
        # condition, même si aucun fichier local n'a changé récemment.
        try:
            force_file = os.path.expanduser("~/.config/rclone/.force-sync")
            os.makedirs(os.path.dirname(force_file), exist_ok=True)
            open(force_file, "w").close()
        except Exception:
            pass
        _, err, code = self.cmd(["systemctl", "start", "--no-block", self.svc])
        return code == 0, err

    def cancel(self):
        """Annule une sync en cours (--no-block pour ne pas attendre la fin)."""
        _, err, code = self.cmd(["systemctl", "stop", "--no-block", self.svc])
        return code == 0, err

    def full(self):
        """Retourne toutes les données pour /api/status."""
        with self.lock:
            svc = self.service()
            parsed = self.parse_runs()
            live = self.streamer.get_live()

            # N'afficher le live que si on est en running ou si le streamer détecte une sync
            show_live = (
                svc["state"] == "running"
                or (live is not None and live.get("is_syncing"))
            )

            return {
                "timer": self.timer(),
                "service": svc,
                "disk": self.disk(),
                "runs": parsed["runs"],
                "logs": self.logs(150),
                "live": live if show_live else None,
                "recent_files": parsed["recent_files"],
                "quota": self.quota,
                "kpis": {
                    "total_files": self.count_files(),
                    "avg_speed": parsed["avg_speed"],
                    "conflicts_today": parsed["conflicts_today"],
                    "success_rate_7d": self.success_rate_7d(),
                    "consecutive_failures": parsed["consecutive_failures"],
                    "last_error_msg": parsed["last_error_msg"],
                },
                "ts": datetime.now().isoformat(),
            }


_m = Monitor()
_d = os.path.dirname(os.path.abspath(__file__))


class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_GET(self):
        p = urlparse(self.path).path
        if p == "/api/status":
            self._json(_m.full())
        elif p == "/api/trigger":
            ok, err = _m.trigger()
            self._json({"ok": ok, "error": err})
        elif p == "/api/cancel":
            ok, err = _m.cancel()
            self._json({"ok": ok, "error": err})
        elif p == "/api/bwlimit":
            try:
                env_path = os.path.expanduser("~/.config/rclone/bwlimit.env")
                limit = ""
                if os.path.exists(env_path):
                    with open(env_path, "r") as f:
                        content = f.read()
                        import re
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
                import glob, shutil, re
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
                import re
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
            base = os.path.abspath(GD_DIR)
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
                        with open(FILTERS_PATH, "r") as f:
                            existing = f.read()
                        if existing and not existing.endswith("\n"):
                            prefix = "\n"
                    except OSError:
                        existing = ""
                    with open(FILTERS_PATH, "a") as f:
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
            base = os.path.abspath(GD_DIR)
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
            base = os.path.abspath(GD_DIR)
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
                    with open(FILTERS_PATH, "r") as f:
                        lines = f.readlines()
                    kept, removed = [], 0
                    for ln in lines:
                        if ln.strip() == rule:
                            removed += 1
                        else:
                            kept.append(ln)
                    if removed:
                        with open(FILTERS_PATH, "w") as f:
                            f.writelines(kept)
                    self._json({"ok": True, "removed": removed})
                except Exception as e:
                    self._json({"ok": False, "error": str(e)})
        elif p == "/api/match_rules":
            # Quels motifs d'exclusion s'appliquent à ce chemin ?
            qs = parse_qs(urlparse(self.path).query)
            target = qs.get("path", [""])[0]
            base = os.path.abspath(GD_DIR)
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
            base = os.path.abspath(GD_DIR)
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
                os.path.join(_d, "index.html"), "text/html;charset=utf-8"
            )
        elif p == "/style.css":
            self._file(
                os.path.join(_d, "style.css"), "text/css;charset=utf-8"
            )
        elif p == "/app.js":
            self._file(
                os.path.join(_d, "app.js"), "application/javascript;charset=utf-8"
            )
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
                base = os.path.abspath(GD_DIR)
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
        base = os.path.abspath(GD_DIR)
        tp = os.path.abspath(os.path.join(base, target or ""))
        if tp == base or not tp.startswith(base + os.sep) or not os.path.exists(tp):
            return None
        return tp

    def _safe_rel(self, target):
        """Valide un chemin relatif (dans la base, ≠ base, sans traversée) et le
        renvoie normalisé — sans exiger qu'il existe localement (utile pour le Drive)."""
        base = os.path.abspath(GD_DIR)
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


class ThreadingServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    """Serveur multi-thread : un dry-run long ne bloque plus les autres requêtes."""
    daemon_threads = True
    allow_reuse_address = True


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    t_q = threading.Thread(target=_m.update_quota, daemon=True)
    t_q.start()
    print(f"\033[32m✓ rclone Monitor\033[0m → http://localhost:{PORT}  |  Ctrl+C pour stopper")
    # Écoute sur localhost uniquement : l'API permet d'écrire des fichiers
    # et de lancer des commandes, elle ne doit pas être exposée au réseau.
    with ThreadingServer(("127.0.0.1", PORT), Handler) as s:
        s.serve_forever()
