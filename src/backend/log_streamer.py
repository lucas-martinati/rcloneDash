import threading
import subprocess
import re
import time
from collections import deque
from datetime import datetime
from . import config
from .filters import local_size

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
        for i, ph in enumerate(config.PHASES):
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
                "phases_total": len(config.PHASES),
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

