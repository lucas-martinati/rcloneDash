#!/bin/bash
set -e

# --------------------------------------------------------------------------- #
#  RcloneDash - Script de désinstallation
# --------------------------------------------------------------------------- #

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD=$'\033[1m'; RESET=$'\033[0m'
    GREEN=$'\033[32m'; YELLOW=$'\033[33m'; RED=$'\033[31m'; CYAN=$'\033[36m'; GREY=$'\033[90m'
else
    BOLD=''; RESET=''; GREEN=''; YELLOW=''; RED=''; CYAN=''; GREY=''
fi

ok()     { printf '   %s✔%s %s\n'  "$GREEN"  "$RESET" "$1"; }
info()   { printf '   %s•%s %s\n'  "$CYAN"   "$RESET" "$1"; }
warn()   { printf '   %s!%s %s\n'  "$YELLOW" "$RESET" "$1"; }
detail() { printf '     %s%s%s\n' "$GREY"   "$1" "$RESET"; }

if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    printf '\n%s%s   ✗ Erreur : ce script ne doit PAS être exécuté avec les privilèges root (sudo).%s\n\n' "$BOLD" "$RED" "$RESET" >&2
    exit 1
fi

printf '\n%s%sDésinstallation de RcloneDash...%s\n\n' "$BOLD" "$CYAN" "$RESET"

# 1. Arrêt des services systemd
info "Arrêt et désactivation des services systemd utilisateur..."
systemctl --user stop rclone-bisync.timer 2>/dev/null || true
systemctl --user disable rclone-bisync.timer 2>/dev/null || true
systemctl --user stop rclone-bisync.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/rclone-bisync.service"
rm -f "$HOME/.config/systemd/user/rclone-bisync.timer"
systemctl --user daemon-reload 2>/dev/null || true
ok "Services systemd retirés"

# 2. Suppression du script de garde
if [ -d "$HOME/.local/share/RcloneDash" ]; then
    rm -rf "$HOME/.local/share/RcloneDash"
    ok "Dossier applicatif ~/.local/share/RcloneDash supprimé"
fi

# 3. Suppression du binaire TUI
if [ -f "$HOME/.local/bin/rclonedash" ]; then
    rm -f "$HOME/.local/bin/rclonedash"
    ok "Binaire ~/.local/bin/rclonedash supprimé"
fi

printf '\n%s%sRcloneDash a été désinstallé avec succès.%s\n' "$BOLD" "$GREEN" "$RESET"
detail "Note : Vos configurations et filtres dans ~/.config/rclone/ ont été conservés."
printf '\n'
