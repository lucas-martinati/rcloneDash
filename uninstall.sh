#!/bin/bash
set -e

# --------------------------------------------------------------------------- #
#  RcloneDash - Uninstaller Script
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
    printf '\n%s%s   ✗ Error: This script must NOT be run with root privileges (sudo).%s\n\n' "$BOLD" "$RED" "$RESET" >&2
    exit 1
fi

printf '\n%s%sUninstalling RcloneDash...%s\n\n' "$BOLD" "$CYAN" "$RESET"

# 1. Stop systemd services
info "Stopping and disabling user systemd services..."
systemctl --user stop rclone-bisync.timer 2>/dev/null || true
systemctl --user disable rclone-bisync.timer 2>/dev/null || true
systemctl --user stop rclone-bisync.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/rclone-bisync.service"
rm -f "$HOME/.config/systemd/user/rclone-bisync.timer"
systemctl --user daemon-reload 2>/dev/null || true
ok "User systemd services removed"

# 2. Remove guard script directory
if [ -d "$HOME/.local/share/RcloneDash" ]; then
    rm -rf "$HOME/.local/share/RcloneDash"
    ok "Application directory ~/.local/share/RcloneDash removed"
fi

# 3. Remove TUI binary
if [ -f "$HOME/.local/bin/rclonedash" ]; then
    rm -f "$HOME/.local/bin/rclonedash"
    ok "Binary ~/.local/bin/rclonedash removed"
fi

printf '\n%s%sRcloneDash has been uninstalled successfully.%s\n' "$BOLD" "$GREEN" "$RESET"
detail "Note: Your configuration and exclusion filters in ~/.config/rclone/ have been preserved."
printf '\n'
