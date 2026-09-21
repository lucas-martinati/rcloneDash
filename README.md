# RcloneDash — Web Dashboard (archive)

> **Branche d'archive, non maintenue.** Snapshot de l'interface Web historique de RcloneDash au moment de l'introduction de la version Rust (commit `4922d5a`, 2026-09-17), avec les captures d'écran d'époque. Le développement actif a déménagé sur la branche `main` (interface terminal Rust).

**RcloneDash Web** surveille et pilote vos synchronisations bidirectionnelles (`rclone bisync`) en temps réel depuis un navigateur : tableau de bord, synchronisation à la demande, explorateur de fichiers, exclusions, simulation dry-run et resynchronisation complète (`--resync`).

---

## 🌐 Fonctionnalités

- **Tableau de bord temps réel** : statut du service `rclone-bisync`, 7 cartes KPI (stockage cloud, disque local, fichiers suivis, syncs du jour, vitesse moyenne, conflits, fiabilité 7 jours).
- **Pouls de synchronisation** : dernière sync, filet de sécurité cloud, timer local.
- **Sync en cours en direct** : stepper de phases (listings → diffs → application → mise à jour), barres de progression, transferts et fichiers actifs.
- **Historique des runs** : succès/échecs, durées, détail des fichiers et des erreurs (copiables).
- **Logs en direct** : streaming `journalctl`, filtres par niveau, pause/reprise.
- **Fichiers récents** : badges de statut, recherche instantanée, ouverture via `xdg-open`.
- **Explorateur de fichiers** : navigation, tri, suppression locale avec comparaison Drive (`rclone check`).
- **Exclusions** : règles `gdrive-filters.txt` avec aperçu d'impact avant ajout.
- **Simulation dry-run** : prévisualisation sans rien modifier.
- **Paramètres** : remote, dossier local, timer, filet cloud, limite de débit.
- **Réparation** : resynchronisation complète (`--resync --resync-mode newer`) sur erreur critique bisync.
- **Thèmes** clair/sombre.

| Tableau de bord | Explorateur & filtres |
|---|---|
| ![Web Dashboard](assets/screenshots/web/dashboard.png) | ![Web File Explorer & Filters](assets/screenshots/web/filters.png) |

| Sync en direct | Paramètres |
|---|---|
| ![Web Live Sync](assets/screenshots/web/live_sync.png) | ![Web Settings](assets/screenshots/web/settings.png) |

---

## 🚀 Lancement

Prérequis : Python 3, `rclone` configuré, pip :

```bash
pip install fastapi uvicorn
python3 web/rclone-monitor.py
```

Puis accédez à [http://localhost:8765](http://localhost:8765) (écoute locale uniquement).

Les services systemd (`services/` : timer `rclone-bisync`, garde `rclone-bisync-guard.sh`, unités `rclonedash`) planifient les synchronisations en arrière-plan ; le dashboard ne fait que les piloter et les observer.
