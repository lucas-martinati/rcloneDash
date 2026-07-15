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
#  Vérification des prérequis
# --------------------------------------------------------------------------- #
if ! command -v node >/dev/null 2>&1 || ! command -v npx >/dev/null 2>&1; then
    printf '\n%s%s[Prérequis]%s %sInstallation de Node.js%s\n' "$BOLD" "$BLUE" "$RESET" "$BOLD" "$RESET"
    warn "Node.js n'est pas installé (requis pour la compilation du JS)."
    detail "Votre mot de passe (sudo) peut être demandé pour son installation."
    if command -v apt-get >/dev/null 2>&1; then
        curl -fsSL https://deb.nodesource.com/setup_current.x | sudo -E bash - >/dev/null 2>&1
        sudo apt-get install -y nodejs >/dev/null 2>&1
        ok "Node.js a été installé avec succès."
    else
        err "Impossible d'installer Node.js automatiquement sur ce système."
        err "Veuillez installer Node.js manuellement puis relancer ce script."
        exit 1
    fi
fi

# --------------------------------------------------------------------------- #
#  Étape 1 — Fichiers de l'application + garde de synchronisation
# --------------------------------------------------------------------------- #
TARGET_DIR="$HOME/.local/share/RcloneDash"

step 1 "Copie des fichiers de l'application"
mkdir -p "$TARGET_DIR"
detail "Destination : $TARGET_DIR"
rm -rf "$TARGET_DIR/js" "$TARGET_DIR/app.js"
npx -y esbuild src/js/main.js --bundle --outfile="$TARGET_DIR/app.js" --minify >/dev/null 2>&1
npx -y esbuild src/css/main.css --bundle --minify --outfile="$TARGET_DIR/style.css" >/dev/null 2>&1
cp src/rclone-monitor.py "$TARGET_DIR/"
sed 's|<script type="module" src="js/main.js"></script>|<script src="app.js"></script>|' src/index.html > "$TARGET_DIR/index.html"
cp -r src/backend "$TARGET_DIR/"
cp requirements.txt "$TARGET_DIR/"
python3 -m venv "$TARGET_DIR/venv"
"$TARGET_DIR/venv/bin/pip" install -r "$TARGET_DIR/requirements.txt" >/dev/null 2>&1
ok "Interface et backend copiés, dépendances installées"

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
#  Étape 4 — Service & timer de synchronisation (utilisateur)
# --------------------------------------------------------------------------- #
step 4 "Service & timer de synchronisation"

sed -e "s|__HOME__|$HOME|g" \
    services/rclone-bisync.service.template > "$HOME/.config/systemd/user/rclone-bisync.service"
cp services/rclone-bisync.timer "$HOME/.config/systemd/user/"

systemctl --user daemon-reload
systemctl --user enable --now rclone-bisync.timer >/dev/null 2>&1
ok "Service et timer installés et activés (niveau utilisateur)"

# Création du fichier de config par défaut si inexistant
CONFIG_FILE="$HOME/.config/rclone/dash-config.json"
if [ ! -f "$CONFIG_FILE" ]; then
    cat <<EOF > "$CONFIG_FILE"
{
  "remote": "GoogleDrive:",
  "local_dir": "~/GoogleDrive",
  "timer_interval": "10min"
}
EOF
    info "Fichier de configuration par défaut généré"
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
