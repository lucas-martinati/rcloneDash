# 🚀 RcloneDash - Notes de version (Release Notes)

## [v1.0.1] - Auto-Update, Centralisation des Paramètres & Optimisations UI

Cette version de maintenance et d'évolution apporte l'auto-updater intégré pour rcloneDash afin de simplifier toutes les futures mises à jour, ainsi que des améliorations significatives sur la gestion des paramètres, le défilement et la propreté du code.

---

### 🌟 Nouveautés et Améliorations

#### 1. 🚀 Auto-Update & Nouvelles Commandes CLI
- **Mise à jour en une commande** : `rclonedash --update` (ou `-u`) télécharge automatiquement la dernière version officielle, ajuste les permissions et remplace l'exécutable de manière atomique.
- **Vérification rapide en ligne de commande** : `rclonedash --check-update` interroge GitHub pour vérifier l'existence d'une nouvelle version.
- **Nouvelles options CLI standard** : `rclonedash --version` (`-v`, `-V`) et `rclonedash --help` (`-h`) sans ouvrir l'interface terminal.
- **Notification visuelle dans le TUI** : une tâche asynchrone non-bloquante vérifie au démarrage la disponibilité d'une mise à jour et affiche un badge discret `(🚀 vX.Y.Z available! Run: rclonedash --update)` dans le menu d'accueil, le panneau des paramètres et la fenêtre d'aide.
- **Binaire autonome publié dans les Releases GitHub** : l'artefact direct `rclonedash-linux-x86_64` est désormais fourni aux côtés des archives `.tar.gz` et des paquets `.deb`.

#### 2. ⚙️ Centralisation & Homogénéisation des Paramètres
- **Source unique de vérité (`SettingId`)** : la liste des choix affichée dans la description détaillée correspond désormais strictement et exactement aux options sélectionnables dans l'application (notamment pour les fréquences de synchronisation et les intervalles de timer).
- **Affichage en colonnes propre et équilibré** : les paramètres à choix multiples nombreux s'organisent désormais en colonnes claires pour une lisibilité optimale.
- **Gestion dynamique de la hauteur des fenêtres** : la hauteur des modales s'ajuste automatiquement selon le nombre de paramètres affichés pour éliminer tout débordement en bas d'écran.

#### 3. 🖱️ Précision des Scrollbars & Navigation
- **Glisser-déposer fluide** : calibrage précis du curseur de scrollbar sur le volet des filtres d'exclusion, des logs, de l'historique et de l'explorateur de fichiers.
- **Gestion unifiée des raccourcis** : centralisation des labels et touches actives avec `KeybindingRegistry`.

#### 4. 🧹 Nettoyage Intégral & Zéro Code Mort
- **Suppression complète de tous les `#[allow(dead_code)]`** : zéro avertissement au compilateur, zéro fonction ou variant inutilisé dans le projet.
- **Unification du logo officiel** : suppression de la variante compacte pour conserver partout le grand logo 3D texturé `LOGO_RCLONEDASH`.
- **Suite de tests étendue** : 66 tests unitaires validés avec 100% de réussite.

---

## [v1.0.0] - Version Majeure & Réécriture Rust TUI

Bienvenue dans la première version officielle majeure de **RcloneDash** !  
Cette version 1.0.0 marque une étape charnière : la réécriture complète du frontend et du moteur de monitoring en une interface terminal moderne, ultra-légère et autonome développée en **Rust**, inspirée des standards visuels de `btop++` et de l'ergonomie de `lazygit`.

---

### 🌟 Nouveautés majeures

#### 1. Architecture Rust Native & Haute Performance
- **Zéro dépendance d'exécution** : compilation en un unique binaire natif autonome de moins de 3 Mo (avec LTO et strip).
- **Empreinte mémoire infime** : consommation inférieure à 10 Mo de RAM (contre ~120 Mo pour la pile Web/Node/Python).
- **Moteur asynchrone Tokio** : gestion des événements terminal non bloquante et flux de données en continu.
- **Support complet de la souris** : clics calibrés au pixel près sur tous les boutons, pills, lignes d'historique, fichiers et modales, ainsi que défilement fluide à la molette.

#### 2. Tableau de bord unifié (Vue Unique Dashboard)
- **Bandeau de statut dynamique** :
  - Statut en temps réel du service systemd (`rclone-bisync.service`).
  - Compte à rebours avant la prochaine synchronisation (`rclone-bisync.timer`).
  - Bouton d'action adaptatif : `[ ⟳ Sync ]` au repos, devenant automatiquement `[ ⏹ Arrêter ]` en rouge pendant une synchronisation en cours.
  - Bouton interactif de fréquence de rafraîchissement style btop++ `[- 250ms +]`.
- **Barre des 7 cartes KPI clés** :
  - Espace de stockage Cloud (Google Drive ou distant configuré).
  - Espace disque local (calculé en temps réel via `statvfs`).
  - Nombre total de fichiers suivis.
  - Synchronisations aujourd'hui (succès et erreurs).
  - Vitesse instantanée de transfert rclone (lecture directe des statistiques journal).
  - Conflits détectés dans la journée.
  - Taux de fiabilité sur 7 jours glissants.
- **Panneau Historique & Graphique d'activité** :
  - Mini-graphique en barres Unicode (` ▂▃▄▅▆▇█`) représentant l'historique chronologique et l'état des exécutions.
  - Liste interactive des runs avec sélection détaillée (`Entrée` ou clic pour afficher les fichiers copiés, supprimés, et la durée).
- **Journal de bord en direct (Live Streaming Logs)** :
  - Écoute continue des logs via `journalctl -u rclone-bisync.service -f`.
  - Sélecteur de filtre interactif avec flèches Unicode homogènes : `← Tout →`, `← Fichiers →`, `← Problèmes →`.
  - Coloration syntaxique contextuelle (horodatages, badges INFO, NOTICE, ERROR, chemins de fichiers, transferts).
  - Pause intelligente du défilement automatique avec `Espace` ou molette.
- **Panneau Fichiers récents** :
  - Liste des derniers fichiers synchronisés avec badges de statut (`● Ajouté`, `● Modifié`, `● Supprimé`).
  - Intégration système : `Entrée` pour ouvrir le fichier (`xdg-open`), `Ctrl+Entrée` ou `d` pour ouvrir le dossier contenant dans le gestionnaire de fichiers.

#### 3. Modales et Menus Dédiés
- **Menu Principal style btop++ (`Échap` ou `m`)** :
  - Bannière ASCII art, options rapides et navigation clavier/souris.
- **Menu Paramètres (`o`)** :
  - Choix parmi **6 thèmes visuels modernes** (*Tokyo Night, Catppuccin Mocha, Nord, Gruvbox, Dracula, Monokai Pro*).
  - Configuration de l'intervalle de synchronisation et du filet cloud périodique (avec option `Jamais (Local)`).
  - Gestion de la limitation de bande passante rclone (`bwlimit`).
  - Accès direct au journal complet rclone via le visualiseur système (`journalctl` / `$PAGER`).
- **Simulateur Dry-Run (`d`)** :
  - Exécution en arrière-plan d'une simulation rclone (`--dry-run`) avec restitution détaillée sans impacter vos données.
- **Éditeur de règles d'exclusion (`e`)** :
  - Visualisation et modification des filtres rclone (`gdrive-filters.txt`) directement depuis le terminal.

#### 4. Intégration Robuste avec Systemd & Rclone
- **Garde de synchronisation intelligente (`rclone-bisync-guard.sh`)** :
  - Déclenchement automatique uniquement en cas de modification locale récente, de demande manuelle, ou de filet de sécurité cloud.
  - Détection et nettoyage préventif des verrous orphelins rclone bisync (`*.lck`).
  - Récupération automatique avec `--resync-mode newer` en cas de listes de référence manquantes.
- **Exécution 100% utilisateur (Non-Root)** :
  - Utilisation exclusive des services systemd de session utilisateur (`systemctl --user`), garantissant la sécurité et le respect des permissions de vos dossiers.

---

### 📦 Installation et Mise à jour

#### Option 1 — Paquet Debian / Ubuntu (.deb)
Téléchargez le fichier `.deb` depuis les assets de cette release et installez-le avec `apt` :
```bash
sudo apt install ./rclonedash_1.0.0-1_amd64.deb
```
- Lancez ensuite directement : `rclonedash`
- Pour configurer la synchronisation automatique systemd en arrière-plan : `rclonedash-setup`

#### Option 2 — Script d'installation automatique (.tar.gz ou Git)
Téléchargez l'archive `.tar.gz` de la release ou clonez le dépôt, puis lancez simplement :
```bash
./install.sh
```
Le script configure automatiquement :
- Le binaire `rclonedash` dans `~/.local/bin/`
- Le script de garde dans `~/.local/share/RcloneDash/`
- Les services et timers systemd utilisateur dans `~/.config/systemd/user/`
- Les fichiers de configuration par défaut dans `~/.config/rclone/`

#### Option 3 — Désinstallation propre
- Si installé via paquet `.deb` : `sudo apt remove rclonedash`
- Si installé via script : `./uninstall.sh`

---

### ⌨️ Raccourcis essentiels
- `m` ou `Échap` : Menu principal
- `s` : Lancer une synchronisation
- `c` : Interrompre la synchronisation en cours
- `o` : Ouvrir les paramètres
- `d` : Lancer une simulation dry-run
- `e` : Ouvrir la gestion des filtres
- `f` : Ouvrir l'explorateur de fichiers
- `t` : Changer de thème instantanément
- `-` / `+` : Ajuster la vitesse de rafraîchissement
- `Espace` : Mettre en pause / reprendre les logs
- `q` : Quitter
