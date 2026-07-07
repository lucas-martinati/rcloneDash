import threading
import subprocess
import json
import os
import shutil
import re
import time
from datetime import datetime
from . import config
from .filters import local_size
from .parsing import parse_synced_file
from .log_streamer import LogStreamer

class Monitor:
    """Collecte toutes les données de monitoring rclone-bisync."""

    def __init__(self):
        self.svc = "rclone-bisync"
        self.tmr = "rclone-bisync.timer"
        self.gd = config.GD_DIR
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
        self._timer = None
        self._timer_time = 0
        self._service = None
        self._service_time = 0

    def cmd(self, c, timeout=10):
        """Exécute une commande et retourne (stdout, stderr, returncode)."""
        try:
            r = subprocess.run(c, capture_output=True, text=True, timeout=timeout)
            return r.stdout.strip(), r.stderr.strip(), r.returncode
        except Exception as e:
            return "", str(e), 1

    def timer(self):
        """État du timer systemd, caché 5s (appelé à chaque /api/status)."""
        now = time.time()
        if now - self._timer_time < 5 and self._timer is not None:
            return self._timer
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
        self._timer = s
        self._timer_time = now
        return s

    def service(self):
        """État du service systemd, caché 5s (appelé à chaque /api/status)."""
        now = time.time()
        if now - self._service_time < 5 and self._service is not None:
            return self._service
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
        self._service = s
        self._service_time = now
        return s

    def update_quota(self):
        while True:
            try:
                out = subprocess.check_output(
                    ["rclone", "about", config.REMOTE, "--json"],
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
                parsed = parse_synced_file(l)
                if parsed:
                    fpath, category = parsed
                    cur[{"new": "copied", "modified": "modified", "deleted": "deleted"}[category]] += 1
                    cur["files"] += 1
                    rf = {
                        "action": category, "path": fpath, "time": ts,
                        "size": None if category == "deleted" else local_size(fpath),
                    }
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

