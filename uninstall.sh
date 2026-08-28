#!/bin/bash
set -e

# --------------------------------------------------------------------------- #
#  Couleurs & helpers d'affichage
# --------------------------------------------------------------------------- #
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD=$'\033[1m';  DIM=$'\033[2m';   RESET=$'\033[0m'
    RED=$'\033[31m';  GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m';  GREY=$'\033[90m'
else
    BOLD=''; DIM=''; RESET=''; RED=''; GREEN=''; YELLOW=''; BLUE=''; CYAN=''; GREY=''
fi

ok()      { printf '   %s✔%s %s\n'  "$GREEN"  "$RESET" "$1"; }
info()    { printf '   %s•%s %s\n'  "$CYAN"   "$RESET" "$1"; }
warn()    { printf '   %s!%s %s\n'  "$YELLOW" "$RESET" "$1"; }
err()     { printf '   %s✗%s %s\n'  "$RED"    "$RESET" "$1" >&2; }
detail()  { printf '     %s%s%s\n' "$GREY"   "$1" "$RESET"; }

printf '\n%s%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n' "$BOLD" "$RED" "$RESET"
printf '%s%s  Désinstallation de RcloneDash%s\n' "$BOLD" "$RED" "$RESET"
printf '%s%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n\n' "$BOLD" "$RED" "$RESET"

# 1. Arrêt et suppression des services utilisateur
info "Arrêt des services utilisateur (rclonedash & rclone-bisync)..."
systemctl --user stop rclonedash.service rclone-bisync.timer rclone-bisync.service 2>/dev/null || true
systemctl --user disable rclonedash.service rclone-bisync.timer 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/rclonedash.service"
rm -f "$HOME/.config/systemd/user/rclone-bisync.service"
rm -f "$HOME/.config/systemd/user/rclone-bisync.timer"
systemctl --user daemon-reload
ok "Services et timers utilisateur supprimés."

# 2. Arrêt et suppression des services système (sudo requis)
if [ -f "/etc/systemd/system/rclone-bisync.timer" ] || [ -f "/etc/systemd/system/rclone-bisync.service" ]; then
    warn "Arrêt des services système rclone-bisync (Droits administrateur requis)..."
    detail "Votre mot de passe (sudo) peut être demandé."
    
    sudo systemctl stop rclone-bisync.timer 2>/dev/null || true
    sudo systemctl disable rclone-bisync.timer 2>/dev/null || true
    sudo rm -f /etc/systemd/system/rclone-bisync.timer
    sudo rm -f /etc/systemd/system/rclone-bisync.service
    sudo systemctl daemon-reload
    ok "Services système rclone-bisync supprimés."
else
    info "Services système rclone-bisync non trouvés."
fi

# 3. Suppression des fichiers locaux (Frontend / Backend)
TARGET_DIR="$HOME/.local/share/RcloneDash"
if [ -d "$TARGET_DIR" ]; then
    rm -rf "$TARGET_DIR"
    ok "Fichiers de l'application supprimés ($TARGET_DIR)."
else
    info "Dossier de l'application non trouvé."
fi

# 4. Suppression des filtres rclone
if [ -f "$HOME/.config/rclone/gdrive-filters.txt" ]; then
    rm -f "$HOME/.config/rclone/gdrive-filters.txt"
    ok "Fichier de filtres rclone supprimé."
fi

printf '\n%s%s  ✔ Désinstallation terminée proprement !%s\n\n' "$BOLD" "$GREEN" "$RESET"
