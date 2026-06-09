# RcloneDash

**RcloneDash** est une interface web locale puissante, interactive et élégante pour surveiller vos synchronisations bidirectionnelles (`rclone bisync`), notamment avec Google Drive.

Ce tableau de bord se branche directement sur les journaux de `rclone` (via `journalctl`) pour fournir une visualisation en temps réel des transferts de fichiers, un historique interactif, la gestion de vos règles de filtrage (fichiers ignorés), et l'exploration directe de votre espace de stockage synchronisé en local.

## 🌟 Fonctionnalités

- **Suivi en direct** : Visualisez l'avancement de votre synchronisation Rclone en temps réel (fichiers transférés, ETA, vitesse de transfert en direct, checks locaux/distants).
- **Historique Interactif** : Lisez le résumé de toutes vos exécutions (fichiers copiés, modifiés, supprimés, erreurs) avec un mini-graphe temporel dynamique (sparkline).
- **Gestion des Erreurs et Logs détaillés** : Un clic sur une exécution passée déroule la liste précise de tous les fichiers affectés et affiche les logs d'erreurs éventuels de ce run spécifique.
- **Éditeur de Filtres Intégré** : Lisez, ajoutez et modifiez directement depuis l'interface graphique votre fichier de filtres global `gdrive-filters.txt`.
- **Navigateur de fichiers local** : Explorez l'arborescence de votre dossier synchronisé en direct. Ouvrez vos fichiers locaux d'un simple clic et excluez rapidement des fichiers pour vos prochains syncs en cliquant sur le bouton "Ignorer".
- **Thème Sombre & Clair** : S'adapte à vos préférences visuelles en un seul clic.

## 🚀 Installation Automatique

Le projet inclut un script d'installation (`install.sh`) qui déplace l'application au bon endroit (`~/.local/share/RcloneDash`) et crée un service `systemd` afin de faire tourner l'interface en tâche de fond automatiquement dès l'allumage du PC. 
Le script génère et configure également de A à Z le timer de synchronisation globale (`rclone-bisync.service` et `rclone-bisync.timer`) dans `/etc/systemd/system/` pour automatiser vos transferts.

```bash
chmod +x install.sh
./install.sh
```

Une fois installé, l'application tourne silencieusement. Rendez-vous sur [http://localhost:8765](http://localhost:8765).

## 🛠 Lancement Manuel

Si vous préférez exécuter le tableau de bord à la demande sans l'installer de façon persistante en tâche de fond :

```bash
python3 src/rclone-monitor.py
```
*Prérequis : Vous devez avoir Python 3 installé sur votre machine (aucune bibliothèque tierce externe n'est nécessaire).*

## ⚙️ Comment ça marche ?

- **Backend** : `rclone-monitor.py` est un serveur HTTP ultra-léger développé en Python natif (`http.server`). Il sert l'interface web, lit vos logs via des commandes Unix (`journalctl`) pour extraire les métriques Rclone, et agit en API pour lire/écrire dans vos fichiers système.
- **Frontend** : Les fichiers `index.html`, `style.css` et `app.js` génèrent l'interface, dessinée entièrement avec du CSS natif pour un design moderne (micro-animations, fenêtres modales, etc.).