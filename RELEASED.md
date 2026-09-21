# 🚀 RcloneDash - Notes de version (Release Notes)

## [v1.0.31] - Notifications Interactives, Icône Vectorielle Officielle & Harmonisation Système

Cette version apporte des notifications de bureau interactives en cas d'échec de synchronisation avec ouverture directe du TUI au clic, une nouvelle identité visuelle avec l'icône vectorielle officielle **Dual-Sync**, l'intégration complète du raccourci de bureau (`.desktop`), un système de mise à jour exhaustif préservant les données utilisateurs, et l'option « Jamais » pour suspendre le timer de synchronisation automatique.

---

### 🌟 Nouveautés et Améliorations

#### 1. 🔔 Notifications de Bureau Interactives (Desktop Notifications)
- **Notification critique immédiate** : alerte de bureau native déclenchée automatiquement par systemd uniquement en cas d'échec de `rclone bisync`.
- **Ouverture directe au clic** : cliquer sur le corps de la notification ou sur l'action « View Dashboard » ouvre instantanément RcloneDash dans votre émulateur de terminal favori (`ptyxis`, `gnome-terminal`, `konsole`, `alacritty`, `kitty`, `xfce4-terminal`, etc.).
- **Résilience système & Contournement AppArmor** : utilisation des liaisons natives GObject/libnotify (`gi.repository.Notify`) pour contourner le confinement AppArmor d'Ubuntu 24.04+, et détachement via `systemd-run --user` garantissant que la notification survit à l'arrêt du cgroup de service (`KillMode=control-group`).
- **Zéro ressource en tâche de fond** : aucun processus résident ni boucle de surveillance polling (0.00% CPU et 0 Mo RAM en fonctionnement normal).

#### 2. 🎨 Nouvelle Identité Visuelle & Raccourci Desktop
- **Icône vectorielle officielle SVG Dual-Sync** : design contemporain à double flèche rouge néon dynamique (`#ff5252` → `#e62525` → `#9e1313`) sur fond transparent avec lueur intégrée, parfaitement contrastée sur docks sombres et clairs.
- **Raccourci applicatif `.desktop`** : intégration dans le lanceur d'applications, le dock et la recherche système (`rclonedash.desktop`), avec catégorie Utilitaire et mots-clés de recherche.
- **Titre de fenêtre explicite** : définition systématique du titre `RcloneDash` sur les fenêtres et onglets de terminal via `crossterm::terminal::SetTitle` et les drapeaux d'émulateurs (`--title` / `-T`).

#### 3. 📦 Mise à Jour Complète & Préservation des Réglages
- **Auto-Updater global (`rclonedash --update`)** : téléchargement de l'archive officielle `.tar.gz` et exécution de l'installateur complet pour mettre à jour atomiquement tous les composants (binaire, unités systemd, icône SVG, scripts de notification).
- **Préservation intégrale des configurations** : l'installateur et l'updater ne réécrivent jamais sur vos réglages existants (`dash-config.json`) ni sur vos filtres d'exclusion (`gdrive-filters.txt`).
- **Gestion automatique des dépendances** : détection et proposition d'installation automatique des dépendances Python requises selon la distribution Linux (APT sur Debian/Ubuntu, DNF sur Fedora, Pacman sur Arch).
- **Désinstallation propre** : script `uninstall.sh` enrichi pour retirer proprement l'icône, le fichier `.desktop` et le cache GTK.

#### 4. ⏱️ Option de suspension du Timer (« Never »)
- **Pause indéfinie du timer** : ajout de l'option `never` dans les paramètres d'intervalle de timer pour suspendre totalement la synchronisation périodique automatique sans désactiver manuellement les services.

#### 5. 🛠️ Harmonisation de l'Installation & Élimination des Doublons
- **Gestion intelligente paquet `.deb` vs installation utilisateur** : `rclonedash-setup` (dans le `.deb`) configure les services sans dupliquer le binaire dans `~/.local/bin` ni le lanceur dans `~/.local/share/applications`.
- **Auto-Updater ciblé (`rclonedash --update`)** : met à jour directement le paquet `.deb` via `apt` s'il est installé au niveau système (`/usr/bin`), ou l'archive utilisateur dans `~/.local/bin`, garantissant une installation propre et sans version fantôme.
- **Nettoyage automatique des résidus** : détection et purge automatique des anciens binaires obsolètes dans `~/.cargo/bin` et `/usr/bin`.

#### 6. 🖥️ Correctif Visuel du Dashboard Vide
- **Affichage complet du logo ASCII** : élargissement de la boîte d'affichage du dashboard vide (de 78 à 86 colonnes) pour éviter la troncature de la lettre « H » de `LOGO_RCLONEDASH`.

---

## [v1.0.2] - Refonte UI Settings (btop++ style), Factorisation & Raccourcis Exposants

Cette version apporte une refonte visuelle majeure du panneau des paramètres inspirée par l'ergonomie et l'esthétique de **btop++**, ainsi qu'une factorisation en profondeur de l'architecture des réglages.

---

### 🌟 Nouveautés et Améliorations

#### 1. 🎨 Refonte de la Modale Settings (Style btop++)
- **Ligne de catégories dédiée & intégrée** : la barre des onglets prend place directement à l'intérieur du conteneur (`tab→ [¹rclone]    ²ui`), séparée du contenu par une ligne horizontale complète avec de véritables jonctions en grille (`├────────────┬────────────┤`).
- **Raccourcis en exposant** : les numéros de bascule rapide d'onglet sont désormais affichés en exposants (`¹`, `²`), en parfaite cohérence avec les cadrans du tableau de bord.
- **Bannière pleine largeur & compteur d'options** : l'élément actif dispose d'un fond bordeaux/marron `#5F1E1E` sur toute la colonne de gauche, avec affichage dynamique du compteur d'options (ex. `Color theme 1/8`, `Auto-Sync Interval 4/7`).
- **Flèches calées aux extrémités** : les flèches `←` et `→` sont positionnées aux bords gauche et droit de la bannière avec la valeur sélectionnée centrée entre elles.
- **Indicateur de pagination intégré** : affichage de `↑ page 1/2 ↓` dans la bordure inférieure sous la colonne des réglages avec activation colorée des flèches si le défilement est possible.

#### 2. ⚙️ Factorisation Complète de l'Architecture des Paramètres
- **Enums canoniques & sources uniques de vérité** : introduction de `SettingCategory` et structuration de `SettingId::ALL` et `SettingId::for_category()` pour éliminer toute duplication d'index ou de comptage en dur.
- **Formattage centralisé** : factorisation de `app.setting_value(setting)` assurant la stricte cohérence entre la colonne de sélection et le panneau de documentation.
- **Boîte de description générique** : remplacement des blocs de match répétés par un dispatch unifié des options cycliques.
- **Sécurité UTF-8** : découpage sécurisé par caractères (`char`) et non plus par octets bruts pour l'ID Google OAuth afin d'éliminer tout risque de panic sur des caractères multi-octets.

---

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
Cette version 1.0.0 marque une étape charnière : la réécriture complète du frontend et du moteur de monitoring en une interface terminal moderne, ultra-légère et autonome développée en **Rust**, inspirée par l'ergonomie de `lazygit`.

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
  - Bouton interactif de fréquence de rafraîchissement `[- 250ms +]`.
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
- **Menu Principal (`Échap` ou `m`)** :
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
