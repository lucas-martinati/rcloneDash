# 🚀 RcloneDash - Notes de version (Release Notes)

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

#### Option 1 — Script d'installation automatique
Téléchargez l'archive de la release ou clonez le dépôt, puis lancez simplement :
```bash
./install.sh
```
Le script configure automatiquement :
- Le binaire `rclonedash` dans `~/.local/bin/`
- Le script de garde dans `~/.local/share/RcloneDash/`
- Les services et timers systemd utilisateur dans `~/.config/systemd/user/`
- Les fichiers de configuration par défaut dans `~/.config/rclone/`

#### Option 2 — Désinstallation propre
Pour désinstaller complètement l'application et ses services systemd :
```bash
./uninstall.sh
```

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
