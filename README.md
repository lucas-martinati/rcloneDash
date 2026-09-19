# RcloneDash (Rust TUI & Web Dashboard)

**RcloneDash** est une interface interactive ultra-rapide, élégante et autonome développée en **Rust** (inspirée de `btop++` et `lazygit`), conçue pour surveiller et piloter vos synchronisations bidirectionnelles (`rclone bisync`) en temps réel.

Le projet propose une puissante **interface terminal (TUI)** moderne (< 3 Mo, < 10 Mo de RAM) tout en conservant son **interface Web** historique (dans le dossier `web/`).

---

## 🚀 Installation & Démarrage Rapide

### Méthode 1 : Paquet Debian / Ubuntu (.deb) — Le plus simple
Téléchargez le fichier `.deb` depuis la page des [Releases GitHub](https://github.com/lucas-martinati/rcloneDash/releases/latest) et installez-le avec `apt` :

```bash
sudo apt install ./rclonedash_*_amd64.deb
```

Une fois installé :
- **Lancer l'interface TUI :**
  ```bash
  rclonedash
  ```
- **Configurer la synchronisation automatique systemd & filtres** *(optionnel, à exécuter dans votre session utilisateur sans `sudo`)* :
  ```bash
  rclonedash-setup
  ```

Pour désinstaller à tout moment :
```bash
sudo apt remove rclonedash
```

---

### Méthode 2 : Installation automatique sans root (Archive release ou Git)
Que vous utilisiez une archive `.tar.gz` issue des **Releases GitHub** ou le dépôt cloné, lancez simplement le script d'installation utilisateur :

```bash
./install.sh
```

Ce script effectue automatiquement et sans privilèges root (`sudo` strictement interdit) :
1. L'installation ou compilation du binaire `rclonedash` dans `~/.local/bin/`
2. La configuration du script de garde intelligent dans `~/.local/share/RcloneDash/`
3. La mise en place des filtres d'exclusion et de la configuration dans `~/.config/rclone/`
4. L'enregistrement et l'activation du timer systemd utilisateur (`rclone-bisync.timer`)

Pour désinstaller proprement à tout moment :
```bash
./uninstall.sh
```

---

### Méthode 3 : Compilation manuelle & Mode développement
```bash
# Compilation optimisée release
cargo build --release
./target/release/rclonedash

# Ou lancement direct en mode développement
cargo run
```

---

## 📸 Captures d'écran & Galerie d'Interfaces

### ⚡ Interface Terminal (Rust TUI)

<!-- Captures d'écran du TUI disponibles dans assets/screenshots/tui/ -->

| Main Dashboard | Interactive Options & Settings |
| :---: | :---: |
| ![Main Dashboard](assets/screenshots/tui/dashboard.png) | ![Settings](assets/screenshots/tui/settings.png) |

| Live Synchronization & Stepper | History Details & Error/Diff Inspection |
| :---: | :---: |
| ![Live Sync](assets/screenshots/tui/live_sync.png) | ![History Details](assets/screenshots/tui/history_details.png) |

| Built-in File Explorer | Exclusion Filters Editor |
| :---: | :---: |
| ![File Explorer](assets/screenshots/tui/files.png) | ![Filters](assets/screenshots/tui/filters.png) |

| Main Menu |
| :---: |
| ![Main Menu](assets/screenshots/tui/menu.png) |

---

### 🌐 Interface Web (Dashboard Navigateur)

| Web Dashboard | Web File Explorer & Filters |
| :---: | :---: |
| ![Web Dashboard](assets/screenshots/web/dashboard.png) | ![Web File Explorer & Filters](assets/screenshots/web/filters.png) |

| Web Live Sync & Transfers | Web Settings |
| :---: | :---: |
| ![Web Live Sync](assets/screenshots/web/live_sync.png) | ![Web Settings](assets/screenshots/web/settings.png) |

*Pour lancer l'interface Web :*
```bash
python3 web/rclone-monitor.py
```
Puis accédez à [http://localhost:8765](http://localhost:8765).

---

## 🌟 Fonctionnalités RcloneDash TUI

- **Tableau de bord tout-en-un centralisé (Vue Unique Dashboard)** :
  - **En-tête dynamique moderne** : statuts du service `rclone-bisync` (actif/en attente/échec), heure et durée de dernière sync, compte à rebours du timer systemd, et boutons interactifs pills (`[ ⟳ Sync ]`, `[ 📁 Files ]`, `[ ⊘ Filters ]`, `[ ⚙ Options ]`, `[ ✕ Quit ]`).
  - **Barre des 7 cartes KPI clés** :
    1. *Stockage Cloud* (ex: Google Drive).
    2. *Disque Local* (calcul `statvfs` en direct : Go utilisés, Go libres et totaux).
    3. *Fichiers suivis* (compte réel des fichiers surveillés).
    4. *Syncs aujourd'hui* (compteur de réussites et erreurs de la journée).
    5. *Débit en direct* (vitesse de transfert instantanée rclone).
    6. *Conflits aujourd'hui* (détection automatique des erreurs dans les logs).
    7. *Fiabilité 7 jours* (taux de réussite sur la semaine glissante).
  - **Panneau Milieu Gauche - Historique & Mini-Graphe** :
    - Mini-graphique en barres Unicode de durée et de statut (` ▂▃▄▅▆▇█`).
    - Tableau interactif des synchronisations passées avec sélection et détails des fichiers copiés, modifiés, supprimés et durées.
  - **Panneau Milieu Droit - Journal de bord en direct (Live Logs)** :
    - Streaming fluide des logs depuis `journalctl` avec coloration syntaxique intelligente.
    - Défilement automatique intelligent ou pause (`Space` / molette).
  - **Panneau Inférieur - Fichiers récemment synchronisés** :
    - Badges colorés de statut (`● Added`, `● Modified`, `● Deleted`), chemins relatifs et horodatage.
- **Menu Principal (Overlay Modal)** :
  - Accessible via `Échap` ou `m` (ou clic sur le bouton `[m]enu`).
  - Grand bandeau ASCII art, version et 3 gros boutons arrondis (`[o] Options`, `[h] Help`, `[q] Quit`).
- **Menu Paramètres (Overlay Modal)** :
  - Sélection parmi 6 thèmes modernes (*Tokyo Night, Catppuccin Mocha, Nord, Gruvbox, Dracula, Monokai Pro*).
  - Réglage de l'intervalle bisync, du filet de sécurité cloud (avec option `Never (Local)` pour désactiver le filet cloud), de la limite `bwlimit`, et de la fréquence de rafraîchissement.
  - Sauvegarde instantanée dans `dash-config.json`.
- **Simulation Dry-Run (Overlay Modal)** :
  - Accessible via `d` ou le bouton `[ 🛡 Dry-Run ]`.
  - Lance un `rclone bisync --dry-run` en tâche de fond pour prévisualiser les transferts sans modifier aucun fichier.
- **Explorateur de fichiers & Historique interactif avec support xdg-open** :
  - Parcourez les fichiers synchronisés ou l'historique complet des runs.
  - `Entrée` ou clic pour ouvrir le fichier dans l'application par défaut.
  - `Ctrl+Entrée`, `d` ou `Ctrl+Clic` pour ouvrir le dossier contenant dans le gestionnaire de fichiers système.
- **Widget de fréquence de rafraîchissement** :
  - Affichage `[- 250ms +]` en haut à droite avec boutons interactifs cliquables et raccourcis clavier (`-` pour accélérer, `+` pour ralentir).
- **Annulation dynamique des synchronisations** :
  - Le bouton du bandeau devient dynamiquement `[ ⏹ Cancel ]` en rouge lorsqu'une synchronisation est active.
  - Raccourci `c` pour annuler immédiatement via `systemctl --user stop rclone-bisync.service`.
- **Calibration pixel-perfect de la souris** :
  - Hitboxes dynamiques recalculées au pixel près pour chaque bouton, ligne de tableau, zone de logs et élément modale.

---

## ⌨️ Raccourcis Clavier & Souris

| Raccourci | Action |
| :--- | :--- |
| `Clic gauche` | Cliquer sur un bouton, un run d'historique, un fichier ou une option |
| `Ctrl + Clic` | Ouvrir le dossier parent du fichier dans l'explorateur système |
| `Molette haut/bas` | Défilement fluide des logs et des listes |
| `Échap` / `m` | Ouvrir le **Menu Principal** (ou fermer la modale active) |
| `o` | Ouvrir les **Paramètres / Options** (ou ouvrir le fichier sélectionné) |
| `d` | Lancer une **Simulation Dry-Run** (ou ouvrir le dossier du fichier sélectionné) |
| `s` / Clic `[ ⟳ Sync ]` | **Forcer une synchronisation** immédiate |
| `c` / Clic `[ ⏹ Cancel ]` | **Interrompre** la synchronisation en cours |
| `r` | Lancer une **Resynchronisation complète** (`--resync`) avec confirmation |
| `f` / Clic `[ 📁 Files ]` | Ouvrir l'**Explorateur de fichiers** local |
| `e` / Clic `[ ⊘ Filters ]` | Ouvrir la **Gestion des règles d'exclusion** |
| `t` | **Changer de thème visuel** à la volée (cycle parmi 6 thèmes) |
| `-` / `+` | **Accélérer** (`-`) ou **ralentir** (`+`) la fréquence de rafraîchissement en ms |
| `Espace` | Mettre en pause / Reprendre le défilement auto des logs |
| `Entrée` | Ouvrir les détails du run ou ouvrir le fichier sélectionné |
| `Ctrl + Entrée` | Ouvrir le **dossier contenant** le fichier sélectionné |
| `Tab` / `Shift+Tab` | Changer de panneau actif (History / Logs / Recent Files) |
| `▲/▼` ou `j/k` | Naviguer dans les lignes du tableau ou des listes |
| `q` | **Quitter silencieusement** l'application |

---

## 🛠 Technologies & Architecture

- **Rust** avec le framework **[Ratatui](https://ratatui.rs/)** pour un rendu terminal 60 FPS sans scintillement.
- **[Tokio](https://tokio.rs/)** pour l'asynchronisme non bloquant du monitoring et des subprocesses.
- **Systemd User Units** (`rclone-bisync.service` et `rclone-bisync.timer`) pour l'automatisation sans aucun privilège root.
- **Interface Web optionnelle** : FastAPI / Python (`rclone-monitor.py`) avec frontend Vanilla HTML5/CSS3/ES Modules dans `web/`.

---

> 💻 *Projet vibe codé avec passion.*

