#!/bin/bash
set -e

# --------------------------------------------------------------------------- #
#  Couleurs & helpers d'affichage
#  Les couleurs sont automatiquement désactivées si la sortie n'est pas un
#  terminal (pipe, redirection) ou si la variable NO_COLOR est définie.
# --------------------------------------------------------------------------- #
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD=$'\033[1m';  DIM=$'\033[2m';   RESET=$'\033[0m'
    RED=$'\033[31m';  GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m';  GREY=$'\033[90m'
else
    BOLD=''; DIM=''; RESET=''; RED=''; GREEN=''; YELLOW=''; BLUE=''; CYAN=''; GREY=''
fi

TOTAL_STEPS=4

# En-tête d'étape :  step <numéro> <titre>
step()    { printf '\n%s%s[%s/%s]%s %s%s%s\n' "$BOLD" "$BLUE" "$1" "$TOTAL_STEPS" "$RESET" "$BOLD" "$2" "$RESET"; }
ok()      { printf '   %s✔%s %s\n'  "$GREEN"  "$RESET" "$1"; }
info()    { printf '   %s•%s %s\n'  "$CYAN"   "$RESET" "$1"; }
warn()    { printf '   %s!%s %s\n'  "$YELLOW" "$RESET" "$1"; }
err()     { printf '   %s✗%s %s\n'  "$RED"    "$RESET" "$1" >&2; }
detail()  { printf '     %s%s%s\n' "$GREY"   "$1" "$RESET"; }

banner() {
    local rule="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf '\n%s%s%s%s\n'      "$BOLD" "$CYAN" "$rule" "$RESET"
    printf '%s%s  Installation de RcloneDash%s\n' "$BOLD" "$CYAN" "$RESET"
    printf '%s%s%s%s\n\n'      "$BOLD" "$CYAN" "$rule" "$RESET"
}

# En cas d'échec, on affiche un message clair plutôt qu'un arrêt silencieux.
trap 'err "Installation interrompue (une commande a échoué)."; exit 1' ERR

banner

# --------------------------------------------------------------------------- #
#  Étape 1 — Fichiers de l'application + garde de synchronisation
# --------------------------------------------------------------------------- #
TARGET_DIR="$HOME/.local/share/RcloneDash"

step 1 "Copie des fichiers de l'application"
mkdir -p "$TARGET_DIR"
detail "Destination : $TARGET_DIR"
rm -rf "$TARGET_DIR/js"
cp -r src/js "$TARGET_DIR/js"
cat src/css/*.css > "$TARGET_DIR/style.css"
cp src/rclone-monitor.py src/index.html "$TARGET_DIR/"
cp -r src/backend "$TARGET_DIR/"
ok "Interface et backend copiés"

# Garde légère : ne lance le bisync que si c'est utile (changement local
# récent, sync périodique, ou déclenchement manuel).
sed -e "s|__HOME__|$HOME|g" services/rclone-bisync-guard.sh.template > "$TARGET_DIR/rclone-bisync-guard.sh"
chmod +x "$TARGET_DIR/rclone-bisync-guard.sh"
ok "Garde de synchronisation installée"

# --------------------------------------------------------------------------- #
#  Étape 2 — Service systemd du Dashboard (utilisateur)
# --------------------------------------------------------------------------- #
step 2 "Service systemd du Dashboard"
systemctl --user stop rclonedash.service >/dev/null 2>&1 || true
sleep 1
mkdir -p "$HOME/.config/systemd/user"
cp services/rclonedash.service "$HOME/.config/systemd/user/"
systemctl --user daemon-reload
systemctl --user enable rclonedash.service >/dev/null 2>&1
systemctl --user start rclonedash.service
ok "RcloneDash démarré en arrière-plan"

# --------------------------------------------------------------------------- #
#  Étape 3 — Filtres rclone
# --------------------------------------------------------------------------- #
step 3 "Configuration des filtres rclone"
mkdir -p "$HOME/.config/rclone"
if [ ! -f "$HOME/.config/rclone/gdrive-filters.txt" ]; then
    cp services/gdrive-filters.txt "$HOME/.config/rclone/"
    ok "Fichier de filtres par défaut installé"
else
    info "gdrive-filters.txt déjà présent — conservé tel quel"
fi

# --------------------------------------------------------------------------- #
#  Étape 4 — Service & timer de synchronisation (système, sudo requis)
# --------------------------------------------------------------------------- #
step 4 "Service & timer de synchronisation"

sed -e "s|__USER__|$USER|g" -e "s|__HOME__|$HOME|g" \
    services/rclone-bisync.service.template > /tmp/rclone-bisync.service

warn "Installation dans /etc/systemd/system/ — droits administrateur requis"
detail "Votre mot de passe (sudo) peut vous être demandé ci-dessous."

# On désactive temporairement le trap ERR pour gérer nous-mêmes l'échec de sudo.
trap - ERR
if sudo cp /tmp/rclone-bisync.service /etc/systemd/system/rclone-bisync.service \
   && sudo cp services/rclone-bisync.timer /etc/systemd/system/rclone-bisync.timer; then
    sudo systemctl daemon-reload
    sudo systemctl enable --now rclone-bisync.timer >/dev/null 2>&1
    ok "Service et timer installés et activés"
else
    err "Impossible de copier les fichiers dans /etc/systemd/system/ (droits refusés)."
    exit 1
fi

# --------------------------------------------------------------------------- #
#  Récapitulatif
# --------------------------------------------------------------------------- #
printf '\n%s%s  ✔ Installation terminée !%s\n' "$BOLD" "$GREEN" "$RESET"
printf '\n'
printf '   %sInterface%s   %s%shttp://localhost:8765%s\n' "$BOLD" "$RESET" "$BOLD" "$CYAN" "$RESET"
printf '   %sLancement%s   automatique à chaque démarrage (dashboard + sync)\n' "$BOLD" "$RESET"
printf '   %sSync%s        bisync déclenché uniquement si un fichier a changé\n' "$BOLD" "$RESET"
printf '               %srécemment, ou toutes les heures pour le cloud%s\n' "$GREY" "$RESET"
printf '\n'
