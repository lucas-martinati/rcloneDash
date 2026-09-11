#!/bin/bash
set -eEo pipefail

# --------------------------------------------------------------------------- #
#  Fichier journal d'installation
# --------------------------------------------------------------------------- #
LOG_FILE="${TMPDIR:-/tmp}/rclonedash-install-$UID.log"
echo "=== Début de l'installation de RcloneDash : $(date) ===" > "$LOG_FILE"

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

# --------------------------------------------------------------------------- #
#  Vérification de l'utilisateur (interdiction de sudo / root)
# --------------------------------------------------------------------------- #
if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    printf '\n%s%s   ✗ Erreur : ce script ne doit PAS être exécuté avec les privilèges root (sudo).%s\n' "$BOLD" "$RED" "$RESET" >&2
    printf '     %sRcloneDash s'\''installe dans votre session utilisateur (~/.local/share, ~/.config/systemd/user).%s\n' "$GREY" "$RESET" >&2
    printf '     %sL'\''exécuter avec sudo empêcherait les services utilisateur systemd de fonctionner.%s\n' "$GREY" "$RESET" >&2
    printf '     %sSi des privilèges administrateur sont nécessaires (ex: Node.js), ils vous seront demandés ponctuellement.%s\n\n' "$GREY" "$RESET" >&2
    printf '     %s%s➜ Relancez la commande sans sudo :%s %s./install.sh%s\n\n' "$BOLD" "$YELLOW" "$RESET" "$BOLD" "$RESET" >&2
    exit 1
fi

banner() {
    local rule="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf '\n%s%s%s%s\n'      "$BOLD" "$CYAN" "$rule" "$RESET"
    printf '%s%s  Installation de RcloneDash%s\n' "$BOLD" "$CYAN" "$RESET"
    printf '%s%s%s%s\n\n'      "$BOLD" "$CYAN" "$rule" "$RESET"
}

# --------------------------------------------------------------------------- #
#  Gestionnaire d'erreurs détaillé
# --------------------------------------------------------------------------- #
on_error() {
    local exit_code="$1"
    local line_no="$2"
    local command="$3"

    printf '\n' >&2
    err "L'installation a échoué (code d'erreur: $exit_code)"
    detail "Commande en échec : $command"
    detail "Ligne : $line_no dans $0"

    if [ -f "$LOG_FILE" ] && [ -s "$LOG_FILE" ]; then
        printf '\n   %s%sDernières lignes du journal :%s\n' "$BOLD" "$YELLOW" "$RESET" >&2
        tail -n 15 "$LOG_FILE" | while IFS= read -r line; do
            printf '     %s│%s %s\n' "$GREY" "$RESET" "$line" >&2
        done
        printf '\n   %sConsultez le journal complet : %s%s\n' "$GREY" "$LOG_FILE" "$RESET" >&2
    fi

    # Diagnostics contextualisés selon la commande qui a échoué
    if [[ "$command" =~ systemctl ]]; then
        printf '\n   %sDiagnostic systemd :%s\n' "$YELLOW" "$RESET" >&2
        if ! systemctl --user status >/dev/null 2>&1; then
            detail "Le gestionnaire de services utilisateur systemd ne semble pas accessible."
            detail "Assurez-vous que votre session utilisateur systemd est active."
            detail "Si vous êtes connecté via SSH ou WSL, essayez d'activer le mode linger :"
            detail "  sudo loginctl enable-linger $USER"
        else
            detail "Le service n'a pas pu démarrer ou s'enregistrer."
            detail "Vérifiez les journaux du service avec :"
            detail "  journalctl --user -u rclonedash.service -n 30 --no-pager"
        fi
    elif [[ "$command" =~ (esbuild|npx|node) ]]; then
        printf '\n   %sDiagnostic Node / esbuild :%s\n' "$YELLOW" "$RESET" >&2
        detail "La compilation du frontend JavaScript/CSS a échoué."
        detail "Vérifiez votre connexion internet ou la présence de Node.js (node -v, npx -v)."
    elif [[ "$command" =~ (pip|venv|python) ]]; then
        printf '\n   %sDiagnostic Python :%s\n' "$YELLOW" "$RESET" >&2
        detail "La configuration de l'environnement virtuel Python a échoué."
        detail "Assurez-vous que python3 et python3-venv sont installés :"
        detail "  sudo apt-get install -y python3 python3-venv python3-pip"
    fi

    printf '\n' >&2
    exit "$exit_code"
}

trap 'on_error "$?" "$LINENO" "$BASH_COMMAND"' ERR

banner

# --------------------------------------------------------------------------- #
#  Vérification des prérequis
# --------------------------------------------------------------------------- #
# 1. Node.js & npx
if ! command -v node >/dev/null 2>&1 || ! command -v npx >/dev/null 2>&1; then
    printf '\n%s%s[Prérequis]%s %sInstallation de Node.js%s\n' "$BOLD" "$BLUE" "$RESET" "$BOLD" "$RESET"
    warn "Node.js n'est pas installé (requis pour la compilation du JS)."
    detail "Votre mot de passe (sudo) peut être demandé pour son installation."
    if command -v apt-get >/dev/null 2>&1; then
        curl -fsSL https://deb.nodesource.com/setup_current.x | sudo -E bash - >> "$LOG_FILE" 2>&1
        sudo apt-get install -y nodejs >> "$LOG_FILE" 2>&1
        ok "Node.js a été installé avec succès."
    else
        err "Impossible d'installer Node.js automatiquement sur ce système (apt-get non trouvé)."
        detail "Veuillez installer Node.js manuellement puis relancer ce script."
        exit 1
    fi
fi

# 2. Python 3
if ! command -v python3 >/dev/null 2>&1; then
    err "Python 3 n'est pas installé sur ce système."
    detail "RcloneDash nécessite Python 3 pour exécuter son serveur backend."
    detail "Installez-le avec : sudo apt-get install -y python3"
    exit 1
fi

# 3. Python 3 venv
if ! python3 -m venv --help >/dev/null 2>&1; then
    err "Le module 'venv' de Python 3 est manquant."
    detail "Sur Debian/Ubuntu, ce composant nécessite un paquet dédié."
    detail "Installez-le avec : sudo apt-get install -y python3-venv"
    exit 1
fi

# 4. Rclone (avertissement informatif)
if ! command -v rclone >/dev/null 2>&1; then
    warn "rclone n'est pas détecté dans votre PATH."
    detail "RcloneDash a besoin de rclone pour synchroniser vos fichiers."
    detail "Installation recommandée : https://rclone.org/install/ ou 'sudo apt install rclone'"
fi

# --------------------------------------------------------------------------- #
#  Étape 1 — Fichiers de l'application + garde de synchronisation
# --------------------------------------------------------------------------- #
TARGET_DIR="$HOME/.local/share/RcloneDash"

step 1 "Copie des fichiers de l'application"
mkdir -p "$TARGET_DIR"
detail "Destination : $TARGET_DIR"
rm -rf "$TARGET_DIR/js" "$TARGET_DIR/app.js"

detail "Compilation du code JavaScript..."
npx -y esbuild src/js/main.js --bundle --outfile="$TARGET_DIR/app.js" --minify >> "$LOG_FILE" 2>&1

detail "Compilation du style CSS..."
npx -y esbuild src/css/main.css --bundle --minify --outfile="$TARGET_DIR/style.css" >> "$LOG_FILE" 2>&1

cp src/rclone-monitor.py "$TARGET_DIR/"
sed 's|<script type="module" src="js/main.js"></script>|<script src="app.js"></script>|' src/index.html > "$TARGET_DIR/index.html"
cp -r src/backend "$TARGET_DIR/"
cp requirements.txt "$TARGET_DIR/"

detail "Configuration de l'environnement virtuel Python..."
python3 -m venv "$TARGET_DIR/venv" >> "$LOG_FILE" 2>&1
"$TARGET_DIR/venv/bin/pip" install -r "$TARGET_DIR/requirements.txt" >> "$LOG_FILE" 2>&1
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
systemctl --user stop rclonedash.service >> "$LOG_FILE" 2>&1 || true
sleep 1
mkdir -p "$HOME/.config/systemd/user"
cp services/rclonedash.service "$HOME/.config/systemd/user/"
systemctl --user daemon-reload >> "$LOG_FILE" 2>&1
systemctl --user enable rclonedash.service >> "$LOG_FILE" 2>&1
systemctl --user start rclonedash.service >> "$LOG_FILE" 2>&1
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

# Création du fichier de config par défaut si inexistant, ou conservation du timer configuré
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
else
    # Restaurer l'intervalle personnalisé déjà configuré
    CUSTOM_INTERVAL=$(python3 -c "import sys, json; print(json.load(open(sys.argv[1])).get('timer_interval', '10min'))" "$CONFIG_FILE" 2>/dev/null || echo "10min")
    if [ -n "$CUSTOM_INTERVAL" ]; then
        sed -i "s|OnUnitActiveSec=.*|OnUnitActiveSec=$CUSTOM_INTERVAL|" "$HOME/.config/systemd/user/rclone-bisync.timer"
    fi
fi

systemctl --user daemon-reload >> "$LOG_FILE" 2>&1
systemctl --user enable --now rclone-bisync.timer >> "$LOG_FILE" 2>&1
systemctl --user restart rclone-bisync.timer >> "$LOG_FILE" 2>&1
ok "Service et timer installés et activés (niveau utilisateur)"

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
