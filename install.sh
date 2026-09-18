#!/bin/bash
set -eEo pipefail

# --------------------------------------------------------------------------- #
#  RcloneDash - Script d'installation unifié (TUI & Services Systemd)
# --------------------------------------------------------------------------- #

LOG_FILE="${TMPDIR:-/tmp}/rclonedash-install-$UID.log"
echo "=== Début de l'installation de RcloneDash : $(date) ===" > "$LOG_FILE"

# Couleurs & helpers
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    BOLD=$'\033[1m';  DIM=$'\033[2m';   RESET=$'\033[0m'
    RED=$'\033[31m';  GREEN=$'\033[32m'; YELLOW=$'\033[33m'
    BLUE=$'\033[34m'; CYAN=$'\033[36m';  GREY=$'\033[90m'
else
    BOLD=''; DIM=''; RESET=''; RED=''; GREEN=''; YELLOW=''; BLUE=''; CYAN=''; GREY=''
fi

TOTAL_STEPS=4

step()    { printf '\n%s%s[%s/%s]%s %s%s%s\n' "$BOLD" "$BLUE" "$1" "$TOTAL_STEPS" "$RESET" "$BOLD" "$2" "$RESET"; }
ok()      { printf '   %s✔%s %s\n'  "$GREEN"  "$RESET" "$1"; }
info()    { printf '   %s•%s %s\n'  "$CYAN"   "$RESET" "$1"; }
warn()    { printf '   %s!%s %s\n'  "$YELLOW" "$RESET" "$1"; }
err()     { printf '   %s✗%s %s\n'  "$RED"    "$RESET" "$1" >&2; }
detail()  { printf '     %s%s%s\n' "$GREY"   "$1" "$RESET"; }

# --------------------------------------------------------------------------- #
#  Vérification de l'utilisateur (interdiction stricte de sudo)
# --------------------------------------------------------------------------- #
if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    printf '\n%s%s   ✗ Erreur : ce script ne doit PAS être exécuté avec les privilèges root (sudo).%s\n' "$BOLD" "$RED" "$RESET" >&2
    printf '     %sRcloneDash s'\''installe dans votre session utilisateur (~/.local/bin, ~/.config/systemd/user).%s\n' "$GREY" "$RESET" >&2
    printf '     %sL'\''exécuter avec sudo empêcherait les services utilisateur systemd de fonctionner.%s\n\n' "$GREY" "$RESET" >&2
    printf '     %s➜ Relancez la commande sans sudo :%s %s./install.sh%s\n\n' "$BOLD" "$YELLOW" "$RESET" "$BOLD" "$RESET" >&2
    exit 1
fi

banner() {
    local rule="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf '\n%s%s%s%s\n'      "$BOLD" "$CYAN" "$rule" "$RESET"
    printf '%s%s  Installation de RcloneDash (Rust TUI & Systemd)%s\n' "$BOLD" "$CYAN" "$RESET"
    printf '%s%s%s%s\n'        "$BOLD" "$CYAN" "$rule" "$RESET"
}

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
    printf '\n' >&2
    exit "$exit_code"
}

trap 'on_error "$?" "$LINENO" "$BASH_COMMAND"' ERR

banner

# Répertoire du script d'installation et détection des modèles
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="$SCRIPT_DIR/services"
if [ ! -f "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" ] && [ -d "/usr/share/rclonedash" ]; then
    TEMPLATE_DIR="/usr/share/rclonedash"
fi

# --------------------------------------------------------------------------- #
#  Vérification des prérequis
# --------------------------------------------------------------------------- #
if ! command -v systemctl >/dev/null 2>&1; then
    err "systemctl n'a pas été trouvé. RcloneDash nécessite systemd pour planifier les synchronisations."
    exit 1
fi

if ! command -v rclone >/dev/null 2>&1; then
    warn "rclone n'est pas détecté dans votre PATH."
    detail "RcloneDash surveille vos synchronisations, mais nécessite rclone pour fonctionner."
    detail "Installez-le avec : sudo apt install rclone (ou https://rclone.org/install/)"
fi

if ! command -v python3 >/dev/null 2>&1; then
    warn "python3 n'est pas détecté dans votre PATH."
    detail "Le script de garde rclone utilise python3 pour lire la configuration JSON."
    detail "Installez-le avec : sudo apt install python3"
fi

# --------------------------------------------------------------------------- #
#  Étape 1 — Installation du binaire TUI
# --------------------------------------------------------------------------- #
step 1 "Installation du binaire RcloneDash TUI"

BIN_SRC=""
if [ -f "$SCRIPT_DIR/rclonedash" ] && [ -x "$SCRIPT_DIR/rclonedash" ]; then
    BIN_SRC="$SCRIPT_DIR/rclonedash"
elif [ -f "$SCRIPT_DIR/target/release/rclonedash" ] && [ -x "$SCRIPT_DIR/target/release/rclonedash" ]; then
    BIN_SRC="$SCRIPT_DIR/target/release/rclonedash"
elif [ -x "/usr/bin/rclonedash" ]; then
    BIN_SRC="/usr/bin/rclonedash"
elif command -v cargo >/dev/null 2>&1 && [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
    info "Compilation du binaire release avec Cargo (optimisations LTO activées)..."
    (cd "$SCRIPT_DIR" && cargo build --release >> "$LOG_FILE" 2>&1)
    BIN_SRC="$SCRIPT_DIR/target/release/rclonedash"
fi

if [ -z "$BIN_SRC" ] || [ ! -f "$BIN_SRC" ]; then
    err "Impossible de localiser ou compiler le binaire rclonedash."
    detail "Si vous installez depuis les sources, assurez-vous d'avoir Rust installé (cargo build --release)."
    detail "Si vous utilisez une archive release, assurez-vous que le fichier 'rclonedash' est présent."
    exit 1
fi

INSTALL_BIN_DIR="$HOME/.local/bin"
if [ "$BIN_SRC" = "/usr/bin/rclonedash" ]; then
    ok "Binaire système détecté dans /usr/bin/rclonedash"
else
    mkdir -p "$INSTALL_BIN_DIR"
    install -m 755 "$BIN_SRC" "$INSTALL_BIN_DIR/rclonedash"
    ok "Binaire installé dans $INSTALL_BIN_DIR/rclonedash"

    # Vérification du PATH
    if [[ ":$PATH:" != *":$INSTALL_BIN_DIR:"* ]]; then
        warn "$INSTALL_BIN_DIR n'est pas dans votre variable PATH !"
        detail "Pour lancer 'rclonedash' directement depuis n'importe où, ajoutez la ligne suivante"
        detail "dans votre fichier ~/.bashrc ou ~/.zshrc :"
        detail "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
fi

# --------------------------------------------------------------------------- #
#  Étape 2 — Installation du script de garde bisync
# --------------------------------------------------------------------------- #
step 2 "Installation du script de garde rclone-bisync"

DATA_DIR="$HOME/.local/share/RcloneDash"
mkdir -p "$DATA_DIR"

if [ -f "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" ]; then
    sed -e "s|__HOME__|$HOME|g" "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" > "$DATA_DIR/rclone-bisync-guard.sh"
    chmod +x "$DATA_DIR/rclone-bisync-guard.sh"
    ok "Script de garde configuré dans $DATA_DIR/rclone-bisync-guard.sh"
else
    err "Modèle 'rclone-bisync-guard.sh.template' introuvable dans $TEMPLATE_DIR."
    exit 1
fi

# --------------------------------------------------------------------------- #
#  Étape 3 — Configuration & Règles d'exclusion rclone
# --------------------------------------------------------------------------- #
step 3 "Configuration et filtres rclone"

RCLONE_CONF_DIR="$HOME/.config/rclone"
mkdir -p "$RCLONE_CONF_DIR"

# gdrive-filters.txt
if [ ! -f "$RCLONE_CONF_DIR/gdrive-filters.txt" ]; then
    if [ -f "$TEMPLATE_DIR/gdrive-filters.txt" ]; then
        cp "$TEMPLATE_DIR/gdrive-filters.txt" "$RCLONE_CONF_DIR/gdrive-filters.txt"
        ok "Fichier de filtres par défaut installé ($RCLONE_CONF_DIR/gdrive-filters.txt)"
    fi
else
    info "Filtres existants conservés ($RCLONE_CONF_DIR/gdrive-filters.txt)"
fi

# dash-config.json
CONFIG_FILE="$RCLONE_CONF_DIR/dash-config.json"
if [ ! -f "$CONFIG_FILE" ]; then
    cat <<EOF > "$CONFIG_FILE"
{
  "remote": "GoogleDrive:",
  "local_dir": "~/GoogleDrive",
  "timer_interval": "10min"
}
EOF
    ok "Fichier de configuration créé avec le dossier local par défaut ~/GoogleDrive ($CONFIG_FILE)"
else
    # Si le fichier existe déjà, s'assurer que local_dir est bien défini (et jamais vide ou corrompu)
    if command -v python3 >/dev/null 2>&1; then
        python3 - <<PYEOF
import json

config_path = "$CONFIG_FILE"
try:
    with open(config_path, "r", encoding="utf-8") as f:
        data = json.load(f)
except Exception:
    data = {}

local_dir = str(data.get("local_dir", "")).strip()
# Si local_dir est vide, absent ou un chemin invalide de test, initialiser à ~/GoogleDrive
if not local_dir or local_dir == "/home/new/path":
    data["local_dir"] = "~/GoogleDrive"
    with open(config_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
PYEOF
    fi
    info "Fichier de configuration existant conservé ($CONFIG_FILE)"
fi

# --------------------------------------------------------------------------- #
#  Étape 4 — Service & Timer Systemd utilisateur
# --------------------------------------------------------------------------- #
step 4 "Installation et activation des services Systemd utilisateur"

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"

if [ -f "$TEMPLATE_DIR/rclone-bisync.service.template" ]; then
    sed -e "s|__HOME__|$HOME|g" "$TEMPLATE_DIR/rclone-bisync.service.template" > "$SYSTEMD_USER_DIR/rclone-bisync.service"
else
    err "Modèle 'rclone-bisync.service.template' introuvable dans $TEMPLATE_DIR."
    exit 1
fi

if [ -f "$TEMPLATE_DIR/rclone-bisync.timer" ]; then
    cp "$TEMPLATE_DIR/rclone-bisync.timer" "$SYSTEMD_USER_DIR/rclone-bisync.timer"
    
    # Ajuster l'intervalle si configuré dans dash-config.json
    if command -v python3 >/dev/null 2>&1 && [ -f "$CONFIG_FILE" ]; then
        CUSTOM_INTERVAL=$(python3 -c "import sys, json; print(json.load(open(sys.argv[1])).get('timer_interval', '10min'))" "$CONFIG_FILE" 2>/dev/null || echo "10min")
        if [ -n "$CUSTOM_INTERVAL" ] && [ "$CUSTOM_INTERVAL" != "10min" ]; then
            sed -i "s|OnUnitInactiveSec=.*|OnUnitInactiveSec=$CUSTOM_INTERVAL|" "$SYSTEMD_USER_DIR/rclone-bisync.timer"
            info "Intervalle du timer ajusté à $CUSTOM_INTERVAL d'après votre configuration"
        fi
    fi
else
    err "Fichier 'services/rclone-bisync.timer' introuvable."
    exit 1
fi

# Rechargement et activation sans sudo
systemctl --user daemon-reload >> "$LOG_FILE" 2>&1
systemctl --user enable --now rclone-bisync.timer >> "$LOG_FILE" 2>&1
systemctl --user restart rclone-bisync.timer >> "$LOG_FILE" 2>&1
ok "Timer systemd utilisateur 'rclone-bisync.timer' activé et démarré"

# --------------------------------------------------------------------------- #
#  Récapitulatif final
# --------------------------------------------------------------------------- #
printf '\n%s%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n' "$BOLD" "$GREEN" "$RESET"
printf '%s%s  ✔ Installation terminée avec succès !%s\n' "$BOLD" "$GREEN" "$RESET"
printf '%s%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n\n' "$BOLD" "$GREEN" "$RESET"

printf '  %s• Lancer l'\''interface TUI :%s   %s%srclonedash%s\n' "$BOLD" "$RESET" "$BOLD" "$CYAN" "$RESET"
printf '  %s• Binaire installé :%s         %s\n' "$BOLD" "$RESET" "$INSTALL_BIN_DIR/rclonedash"
printf '  %s• Configuration :%s            %s\n' "$BOLD" "$RESET" "$CONFIG_FILE"
printf '  %s• Filtres d'\''exclusion :%s      %s\n' "$BOLD" "$RESET" "$RCLONE_CONF_DIR/gdrive-filters.txt"
printf '  %s• Service de garde :%s         %s\n' "$BOLD" "$RESET" "$DATA_DIR/rclone-bisync-guard.sh"
printf '  %s• Statut du timer :%s          systemctl --user status rclone-bisync.timer\n\n' "$BOLD" "$RESET"
