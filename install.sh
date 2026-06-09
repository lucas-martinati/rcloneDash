#!/bin/bash
set -e

echo "=== Installation de RcloneDash ==="

# Définition du dossier cible
TARGET_DIR="$HOME/.local/share/RcloneDash"
mkdir -p "$TARGET_DIR"

# Copie des fichiers vitaux de l'application
echo "[1] Copie des fichiers vers $TARGET_DIR..."
cp src/rclone-monitor.py src/app.js src/style.css src/index.html "$TARGET_DIR/"

# Création du service systemd utilisateur pour exécuter le serveur web en arrière-plan
echo "[2] Installation du service systemd pour le Dashboard..."
mkdir -p "$HOME/.config/systemd/user"
cp services/rclonedash.service "$HOME/.config/systemd/user/"

# Activation du service RcloneDash (User)
systemctl --user daemon-reload
systemctl --user enable --now rclonedash.service
echo "-> RcloneDash démarré avec succès."

# Installation du fichier de filtres
echo "[3] Configuration des filtres Rclone..."
mkdir -p "$HOME/.config/rclone"
if [ ! -f "$HOME/.config/rclone/gdrive-filters.txt" ]; then
    cp services/gdrive-filters.txt "$HOME/.config/rclone/"
    echo "-> Fichier de filtres par défaut installé."
else
    echo "-> Fichier gdrive-filters.txt déjà existant (non écrasé)."
fi

# Configuration du service de synchronisation système (rclone-bisync)
echo "[4] Création et activation de rclone-bisync.service et rclone-bisync.timer..."

# Personnalisation du template avec l'utilisateur actuel
sed -e "s|__USER__|$USER|g" -e "s|__HOME__|$HOME|g" services/rclone-bisync.service.template > /tmp/rclone-bisync.service

echo "L'installation dans /etc/systemd/system/ nécessite les droits administrateur."
echo "Il vous sera probablement demandé de saisir votre mot de passe (sudo) :"
if sudo cp /tmp/rclone-bisync.service /etc/systemd/system/rclone-bisync.service && sudo cp services/rclone-bisync.timer /etc/systemd/system/rclone-bisync.timer; then
    sudo systemctl daemon-reload
    sudo systemctl enable --now rclone-bisync.timer
    echo "-> Service et Timer de synchronisation installés et activés avec succès."
else
    echo "ERREUR : Impossible de copier les fichiers dans /etc/systemd/system/ (Droits refusés)."
    exit 1
fi

echo ""
echo "=== Installation terminée ! ==="
echo "Vous pouvez accéder à votre interface ici : http://localhost:8765"
echo "Le tableau de bord et le timer de synchronisation se lanceront désormais automatiquement à chaque démarrage."
