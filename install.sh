#!/bin/bash
set -eEo pipefail

# --------------------------------------------------------------------------- #
#  RcloneDash - Unified Installer Script (TUI & Systemd Services)
# --------------------------------------------------------------------------- #

LOG_FILE="${TMPDIR:-/tmp}/rclonedash-install-$UID.log"
echo "=== Starting RcloneDash installation: $(date) ===" > "$LOG_FILE"

# Colors & formatting helpers
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
#  User Verification (strict non-root requirement)
# --------------------------------------------------------------------------- #
if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    printf '\n%s%s   ✗ Error: This script must NOT be run with root privileges (sudo).%s\n' "$BOLD" "$RED" "$RESET" >&2
    printf '     %sRcloneDash installs inside your user session (~/.local/bin, ~/.config/systemd/user).%s\n' "$GREY" "$RESET" >&2
    printf '     %sRunning with sudo will prevent user systemd services from functioning properly.%s\n\n' "$GREY" "$RESET" >&2
    printf '     %s➜ Run the command again without sudo:%s %s./install.sh%s\n\n' "$BOLD" "$YELLOW" "$RESET" "$BOLD" "$RESET" >&2
    exit 1
fi

banner() {
    local rule="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    printf '\n%s%s%s%s\n'      "$BOLD" "$CYAN" "$rule" "$RESET"
    printf '%s%s  Installing RcloneDash (Rust TUI & Systemd)%s\n' "$BOLD" "$CYAN" "$RESET"
    printf '%s%s%s%s\n'        "$BOLD" "$CYAN" "$rule" "$RESET"
}

on_error() {
    local exit_code="$1"
    local line_no="$2"
    local command="$3"

    printf '\n' >&2
    err "Installation failed (exit code: $exit_code)"
    detail "Failed command: $command"
    detail "Line: $line_no in $0"

    if [ -f "$LOG_FILE" ] && [ -s "$LOG_FILE" ]; then
        printf '\n   %s%sLast lines of the installation log:%s\n' "$BOLD" "$YELLOW" "$RESET" >&2
        tail -n 15 "$LOG_FILE" | while IFS= read -r line; do
            printf '     %s│%s %s\n' "$GREY" "$RESET" "$line" >&2
        done
        printf '\n   %sCheck the full log at: %s%s\n' "$GREY" "$LOG_FILE" "$RESET" >&2
    fi
    printf '\n' >&2
    exit "$exit_code"
}

TMP_DL_DIR=""
cleanup() {
    if [ -n "$TMP_DL_DIR" ] && [ -d "$TMP_DL_DIR" ]; then
        rm -rf "$TMP_DL_DIR"
    fi
}
trap cleanup EXIT

trap 'on_error "$?" "$LINENO" "$BASH_COMMAND"' ERR

banner

# Determine script directory and template location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="$SCRIPT_DIR/services"

# If templates are not found locally (e.g. piped execution via curl | bash), download the latest release
if [ ! -f "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" ] && [ ! -d "/usr/share/rclonedash" ]; then
    info "Standalone mode detected: downloading latest official release package..."
    TMP_DL_DIR=$(mktemp -d "${TMPDIR:-/tmp}/rclonedash-install-XXXXXX")

    RELEASE_API="https://api.github.com/repos/lucas-martinati/rcloneDash/releases/latest"
    TAR_URL=$(curl -sL "$RELEASE_API" | grep -o 'https://github.com/lucas-martinati/rcloneDash/releases/download/[^" ]*linux-x86_64.tar.gz' | head -n 1)
    if [ -z "$TAR_URL" ]; then
        TAR_URL="https://github.com/lucas-martinati/rcloneDash/releases/latest/download/rclonedash-v1.0.0-linux-x86_64.tar.gz"
    fi

    if curl -fsSL "$TAR_URL" -o "$TMP_DL_DIR/rclonedash.tar.gz" 2>>"$LOG_FILE"; then
        tar -xzf "$TMP_DL_DIR/rclonedash.tar.gz" -C "$TMP_DL_DIR" --strip-components=1 2>>"$LOG_FILE"
        SCRIPT_DIR="$TMP_DL_DIR"
        TEMPLATE_DIR="$TMP_DL_DIR/services"
        ok "Official release package downloaded and extracted"
    else
        err "Failed to download release archive from $TAR_URL"
        exit 1
    fi
fi

if [ ! -f "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" ] && [ -d "/usr/share/rclonedash" ]; then
    TEMPLATE_DIR="/usr/share/rclonedash"
fi

# --------------------------------------------------------------------------- #
#  Prerequisites Check
# --------------------------------------------------------------------------- #
if ! command -v systemctl >/dev/null 2>&1; then
    err "systemctl was not found. RcloneDash requires systemd to schedule background synchronizations."
    exit 1
fi

if ! command -v rclone >/dev/null 2>&1 && [ ! -x "$HOME/.local/bin/rclone" ]; then
    info "rclone is not detected. Installing rclone automatically..."
    RCLONE_INSTALLED=0
    # 1. Try official installer if sudo without password or root is available
    if command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
        if curl -fsSL https://rclone.org/install.sh | sudo bash >> "$LOG_FILE" 2>&1; then
            RCLONE_INSTALLED=1
            ok "rclone installed system-wide via official installer"
        fi
    fi

    # 2. If not installed, download precompiled standalone binary directly to ~/.local/bin/rclone
    if [ "$RCLONE_INSTALLED" -eq 0 ]; then
        mkdir -p "$HOME/.local/bin"
        ARCH="$(uname -m)"
        case "$ARCH" in
            x86_64)  ARCH_TAG="linux-amd64" ;;
            aarch64) ARCH_TAG="linux-arm64" ;;
            armv7*)  ARCH_TAG="linux-arm-v7" ;;
            *)       ARCH_TAG="linux-amd64" ;;
        esac
        RCLONE_URL="https://downloads.rclone.org/rclone-current-${ARCH_TAG}.zip"
        TMP_ZIP=$(mktemp "${TMPDIR:-/tmp}/rclone-dl-XXXXXX.zip")
        if curl -fsSL "$RCLONE_URL" -o "$TMP_ZIP" 2>>"$LOG_FILE"; then
            if command -v unzip >/dev/null 2>&1; then
                unzip -p "$TMP_ZIP" "*/rclone" > "$HOME/.local/bin/rclone" 2>>"$LOG_FILE"
                chmod 755 "$HOME/.local/bin/rclone"
                RCLONE_INSTALLED=1
            elif command -v python3 >/dev/null 2>&1; then
                python3 -c "import zipfile, sys; z = zipfile.ZipFile('$TMP_ZIP'); f = next(n for n in z.namelist() if n.endswith('/rclone') or n == 'rclone'); sys.stdout.buffer.write(z.read(f))" > "$HOME/.local/bin/rclone" 2>>"$LOG_FILE"
                chmod 755 "$HOME/.local/bin/rclone"
                RCLONE_INSTALLED=1
            fi
            rm -f "$TMP_ZIP"
            if [ "$RCLONE_INSTALLED" -eq 1 ]; then
                ok "rclone standalone binary installed to $HOME/.local/bin/rclone"
            fi
        fi
    fi

    if ! command -v rclone >/dev/null 2>&1 && [ ! -x "$HOME/.local/bin/rclone" ]; then
        warn "Could not automatically install rclone."
        detail "Install it manually with: sudo apt install rclone (or https://rclone.org/install/)"
    fi
else
    RCLONE_BIN="$(command -v rclone 2>/dev/null || echo "$HOME/.local/bin/rclone")"
    ok "rclone detected: $($RCLONE_BIN --version 2>/dev/null | head -n 1)"
fi

if ! command -v python3 >/dev/null 2>&1; then
    warn "python3 is not detected in your PATH."
    detail "The rclone guard script uses python3 to parse configuration and deliver interactive notifications."
    detail "Install it with: sudo apt install python3 (or equivalent package manager)"
else
    ok "python3 detected: $(python3 --version 2>/dev/null)"
    # Check Python GObject & libnotify bindings for interactive error notifications
    if python3 -c "import gi; gi.require_version('Notify', '0.7'); from gi.repository import Notify" >/dev/null 2>&1; then
        ok "Python GObject & libnotify bindings detected (interactive error notifications enabled)"
    else
        info "Python libnotify bindings (python3-gi / gir1.2-notify-0.7) not found."
        detail "Attempting automatic installation for interactive notification support..."
        INSTALL_GI=0
        if command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
            if command -v apt-get >/dev/null 2>&1; then
                sudo apt-get update -qq >> "$LOG_FILE" 2>&1 && \
                sudo apt-get install -y -qq python3-gi gir1.2-notify-0.7 >> "$LOG_FILE" 2>&1 && INSTALL_GI=1
            elif command -v dnf >/dev/null 2>&1; then
                sudo dnf install -y -q python3-gobject libnotify >> "$LOG_FILE" 2>&1 && INSTALL_GI=1
            elif command -v pacman >/dev/null 2>&1; then
                sudo pacman -S --noconfirm --needed python-gobject libnotify >> "$LOG_FILE" 2>&1 && INSTALL_GI=1
            fi
        fi
        if [ "$INSTALL_GI" -eq 1 ] && python3 -c "import gi; gi.require_version('Notify', '0.7'); from gi.repository import Notify" >/dev/null 2>&1; then
            ok "Python libnotify bindings installed successfully"
        else
            warn "Interactive notification libraries missing. Notifications will still work, but without click-to-open."
            detail "To enable click-to-open on notifications, install:"
            detail "  • Ubuntu/Debian: sudo apt install python3-gi gir1.2-notify-0.7"
            detail "  • Fedora:        sudo dnf install python3-gobject libnotify"
            detail "  • Arch Linux:    sudo pacman -S python-gobject libnotify"
        fi
    fi
fi

# --------------------------------------------------------------------------- #
#  Step 1 — Install TUI Binary
# --------------------------------------------------------------------------- #
step 1 "Installing RcloneDash TUI binary"

IS_SYSTEM_SETUP=0
if [ "$(basename "$0")" = "rclonedash-setup" ] || [ -f "/usr/bin/rclonedash-setup" -a -f "/usr/bin/rclonedash" -a ! -f "$SCRIPT_DIR/Cargo.toml" -a ! -f "$SCRIPT_DIR/target/release/rclonedash" -a ! -f "$SCRIPT_DIR/rclonedash" ]; then
    IS_SYSTEM_SETUP=1
fi

if [ "$IS_SYSTEM_SETUP" -eq 1 ]; then
    if [ -x "/usr/bin/rclonedash" ]; then
        ok "Using system binary /usr/bin/rclonedash (Debian package)"
        BIN_PATH="/usr/bin/rclonedash"
        # Clean up any stale duplicate in ~/.local/bin or ~/.cargo/bin
        if [ -f "$HOME/.local/bin/rclonedash" ]; then
            rm -f "$HOME/.local/bin/rclonedash" 2>/dev/null || true
            info "Removed duplicate binary ~/.local/bin/rclonedash"
        fi
        if [ -f "$HOME/.cargo/bin/rclonedash" ]; then
            rm -f "$HOME/.cargo/bin/rclonedash" 2>/dev/null || true
            info "Removed duplicate binary ~/.cargo/bin/rclonedash"
        fi
    else
        err "System binary /usr/bin/rclonedash not found. Reinstall the package via: sudo apt install ./rclonedash*.deb"
        exit 1
    fi
else
    BIN_SRC=""
    if [ -f "$SCRIPT_DIR/rclonedash" ] && [ -x "$SCRIPT_DIR/rclonedash" ]; then
        BIN_SRC="$SCRIPT_DIR/rclonedash"
    elif [ -f "$SCRIPT_DIR/target/release/rclonedash" ] && [ -x "$SCRIPT_DIR/target/release/rclonedash" ]; then
        BIN_SRC="$SCRIPT_DIR/target/release/rclonedash"
    elif command -v cargo >/dev/null 2>&1 && [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
        info "Compiling release binary with Cargo (LTO optimizations enabled)..."
        (cd "$SCRIPT_DIR" && cargo build --release >> "$LOG_FILE" 2>&1)
        BIN_SRC="$SCRIPT_DIR/target/release/rclonedash"
    fi

    if [ -z "$BIN_SRC" ] || [ ! -f "$BIN_SRC" ]; then
        err "Unable to locate or compile the rclonedash binary."
        detail "If compiling from source, make sure Rust/Cargo is installed (cargo build --release)."
        detail "If using a release archive, verify that the 'rclonedash' file is present."
        exit 1
    fi

    INSTALL_BIN_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_BIN_DIR"
    install -m 755 "$BIN_SRC" "$INSTALL_BIN_DIR/rclonedash"
    ok "Binary installed to $INSTALL_BIN_DIR/rclonedash"
    BIN_PATH="$INSTALL_BIN_DIR/rclonedash"

    # Guarantee zero version mismatch: detect and clean up any stale duplicates on the system
    if [ -f "/usr/bin/rclonedash" ]; then
        warn "A stale duplicate system binary was detected at /usr/bin/rclonedash!"
        if command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
            info "Cleaning up old system binary and desktop entry with sudo..."
            sudo rm -f "/usr/bin/rclonedash" "/usr/bin/rclonedash-setup" "/usr/share/applications/rclonedash.desktop" 2>/dev/null || true
            ok "Stale system binary removed"
        elif command -v dpkg >/dev/null 2>&1 && dpkg -s rclonedash >/dev/null 2>&1; then
            detail "To avoid version conflicts, purge the old system package via: sudo apt remove rclonedash"
        else
            detail "To avoid version conflicts, remove the old file via: sudo rm -f /usr/bin/rclonedash"
        fi
    fi

    # Clean up stale ~/.cargo/bin duplicate if present
    if [ -f "$HOME/.cargo/bin/rclonedash" ]; then
        info "Cleaning up old ~/.cargo/bin/rclonedash duplicate..."
        rm -f "$HOME/.cargo/bin/rclonedash" 2>/dev/null || true
    fi

    # PATH verification
    if [[ ":$PATH:" != *":$INSTALL_BIN_DIR:"* ]]; then
        warn "$INSTALL_BIN_DIR is not in your PATH variable!"
        detail "To launch 'rclonedash' directly from anywhere, add the following line"
        detail "to your ~/.bashrc or ~/.zshrc file:"
        detail "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
fi

# --------------------------------------------------------------------------- #
#  Step 2 — Install Bisync Guard Script
# --------------------------------------------------------------------------- #
step 2 "Installing rclone-bisync guard script"

DATA_DIR="$HOME/.local/share/RcloneDash"
mkdir -p "$DATA_DIR"

if [ -f "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" ]; then
    sed -e "s|__HOME__|$HOME|g" "$TEMPLATE_DIR/rclone-bisync-guard.sh.template" > "$DATA_DIR/rclone-bisync-guard.sh"
    chmod +x "$DATA_DIR/rclone-bisync-guard.sh"
    ok "Guard script configured at $DATA_DIR/rclone-bisync-guard.sh"
else
    err "Template 'rclone-bisync-guard.sh.template' not found in $TEMPLATE_DIR."
    exit 1
fi

if [ -f "$TEMPLATE_DIR/rclonedash-notify.py" ]; then
    cp "$TEMPLATE_DIR/rclonedash-notify.py" "$DATA_DIR/rclonedash-notify.py"
    chmod +x "$DATA_DIR/rclonedash-notify.py"
    ok "Notification helper installed at $DATA_DIR/rclonedash-notify.py"
fi

if [ "$IS_SYSTEM_SETUP" -eq 1 ]; then
    # In system package mode, the desktop entry and icon are already in /usr/share/...
    # Clean up any duplicate ~/.local/share/applications/rclonedash.desktop if it was previously created
    if [ -f "$HOME/.local/share/applications/rclonedash.desktop" ]; then
        rm -f "$HOME/.local/share/applications/rclonedash.desktop"
        info "Removed duplicate desktop launcher in ~/.local/share/applications"
    fi
    ok "System desktop launcher active (/usr/share/applications/rclonedash.desktop)"
    ok "System application icon active (/usr/share/icons/hicolor/scalable/apps/rclonedash.svg)"
else
    # Install application icon
    ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
    mkdir -p "$ICON_DIR"
    if [ -f "$TEMPLATE_DIR/rclonedash.svg" ]; then
        cp "$TEMPLATE_DIR/rclonedash.svg" "$ICON_DIR/rclonedash.svg"
        ok "Application icon installed at $ICON_DIR/rclonedash.svg"
        if command -v gtk-update-icon-cache >/dev/null 2>&1; then
            gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >> "$LOG_FILE" 2>&1 || true
        fi
    fi

    # Install desktop file for application launcher and notification integration
    APP_DIR="$HOME/.local/share/applications"
    mkdir -p "$APP_DIR"
    if [ -f "$TEMPLATE_DIR/rclonedash.desktop.template" ]; then
        sed -e "s|__BIN__|$BIN_PATH|g" "$TEMPLATE_DIR/rclonedash.desktop.template" > "$APP_DIR/rclonedash.desktop"
        chmod +x "$APP_DIR/rclonedash.desktop"
        ok "Desktop entry installed at $APP_DIR/rclonedash.desktop"
        if command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database "$APP_DIR" >> "$LOG_FILE" 2>&1 || true
        fi
    else
        info "Desktop template not found — skipping .desktop file installation"
    fi
fi

# --------------------------------------------------------------------------- #
#  Step 3 — Configuration & Rclone Exclusion Filters
# --------------------------------------------------------------------------- #
step 3 "Setting up configuration and rclone filters"

RCLONE_CONF_DIR="$HOME/.config/rclone"
mkdir -p "$RCLONE_CONF_DIR"

# gdrive-filters.txt
if [ ! -f "$RCLONE_CONF_DIR/gdrive-filters.txt" ]; then
    if [ -f "$TEMPLATE_DIR/gdrive-filters.txt" ]; then
        cp "$TEMPLATE_DIR/gdrive-filters.txt" "$RCLONE_CONF_DIR/gdrive-filters.txt"
        ok "Default filters file installed ($RCLONE_CONF_DIR/gdrive-filters.txt)"
    fi
else
    info "Preserving existing filters ($RCLONE_CONF_DIR/gdrive-filters.txt)"
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
    ok "Configuration file created with default local directory ~/GoogleDrive ($CONFIG_FILE)"
else
    # If the file already exists, ensure local_dir is well defined
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
if not local_dir or local_dir == "/home/new/path":
    data["local_dir"] = "~/GoogleDrive"
    with open(config_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
PYEOF
    fi
    info "Preserving existing configuration file ($CONFIG_FILE)"
fi

# --------------------------------------------------------------------------- #
#  Step 4 — User Systemd Service & Timer
# --------------------------------------------------------------------------- #
step 4 "Configuring and enabling user systemd services"

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"

if [ -f "$TEMPLATE_DIR/rclone-bisync.service.template" ]; then
    sed -e "s|__HOME__|$HOME|g" "$TEMPLATE_DIR/rclone-bisync.service.template" > "$SYSTEMD_USER_DIR/rclone-bisync.service"
else
    err "Template 'rclone-bisync.service.template' not found in $TEMPLATE_DIR."
    exit 1
fi

if [ -f "$TEMPLATE_DIR/rclone-bisync.timer" ]; then
    cp "$TEMPLATE_DIR/rclone-bisync.timer" "$SYSTEMD_USER_DIR/rclone-bisync.timer"
    
    # Adjust timer interval if configured in dash-config.json
    if command -v python3 >/dev/null 2>&1 && [ -f "$CONFIG_FILE" ]; then
        CUSTOM_INTERVAL=$(python3 -c "import sys, json; print(json.load(open(sys.argv[1])).get('timer_interval', '10min'))" "$CONFIG_FILE" 2>/dev/null || echo "10min")
        if [ -n "$CUSTOM_INTERVAL" ] && [ "$CUSTOM_INTERVAL" != "10min" ]; then
            sed -i "s|OnUnitInactiveSec=.*|OnUnitInactiveSec=$CUSTOM_INTERVAL|" "$SYSTEMD_USER_DIR/rclone-bisync.timer"
            info "Timer interval adjusted to $CUSTOM_INTERVAL from your configuration"
        fi
    fi
else
    err "File 'services/rclone-bisync.timer' not found."
    exit 1
fi

# Reload and enable without sudo
systemctl --user daemon-reload >> "$LOG_FILE" 2>&1
systemctl --user enable --now rclone-bisync.timer >> "$LOG_FILE" 2>&1
systemctl --user restart rclone-bisync.timer >> "$LOG_FILE" 2>&1
ok "User systemd timer 'rclone-bisync.timer' enabled and started"

# --------------------------------------------------------------------------- #
#  Installation Summary
# --------------------------------------------------------------------------- #
printf '\n%s%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n' "$BOLD" "$GREEN" "$RESET"
printf '%s%s  ✔ Installation completed successfully!%s\n' "$BOLD" "$GREEN" "$RESET"
printf '%s%s━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%s\n\n' "$BOLD" "$GREEN" "$RESET"

printf '  %s• Launch TUI interface:%s   %s%srclonedash%s\n' "$BOLD" "$RESET" "$BOLD" "$CYAN" "$RESET"
printf '  %s• Installed binary:%s       %s\n' "$BOLD" "$RESET" "$INSTALL_BIN_DIR/rclonedash"
printf '  %s• Application icon:%s       %s\n' "$BOLD" "$RESET" "$ICON_DIR/rclonedash.svg"
printf '  %s• Desktop launcher:%s       %s\n' "$BOLD" "$RESET" "$APP_DIR/rclonedash.desktop"
printf '  %s• Configuration:%s          %s\n' "$BOLD" "$RESET" "$CONFIG_FILE"
printf '  %s• Exclusion filters:%s      %s\n' "$BOLD" "$RESET" "$RCLONE_CONF_DIR/gdrive-filters.txt"
printf '  %s• Guard script:%s           %s\n' "$BOLD" "$RESET" "$DATA_DIR/rclone-bisync-guard.sh"
printf '  %s• Notification helper:%s    %s\n' "$BOLD" "$RESET" "$DATA_DIR/rclonedash-notify.py"
printf '  %s• Timer status:%s           systemctl --user status rclone-bisync.timer\n\n' "$BOLD" "$RESET"
