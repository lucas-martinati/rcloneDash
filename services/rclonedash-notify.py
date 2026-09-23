#!/usr/bin/env python3
"""
RcloneDash failure notification with interactive click-to-open TUI.
"""
import os
import sys
import shutil
import subprocess

RCLONE_EXIT_REASONS = {
    "1": "Syntax/Config error",
    "2": "Sync interrupted or conflict detected",
    "3": "Directory not found",
    "4": "File not found",
    "5": "Temporary network / rate-limit error",
    "6": "Partial transfer failure",
    "7": "Fatal synchronization error",
}

def get_rclonedash_bin():
    # Check ~/.local/bin first (standard user installation and updates)
    local_b = os.path.expanduser("~/.local/bin/rclonedash")
    if os.path.isfile(local_b) and os.access(local_b, os.X_OK):
        return local_b
    # Check PATH
    b = shutil.which("rclonedash")
    if b:
        return b
    # Check /usr/bin directly (Debian package install)
    if os.path.isfile("/usr/bin/rclonedash") and os.access("/usr/bin/rclonedash", os.X_OK):
        return "/usr/bin/rclonedash"
    # Check ~/.cargo/bin
    cargo_b = os.path.expanduser("~/.cargo/bin/rclonedash")
    if os.path.isfile(cargo_b) and os.access(cargo_b, os.X_OK):
        return cargo_b
    return "rclonedash"

def open_rclonedash():
    cmd = get_rclonedash_bin()
    print(f"open_rclonedash using binary: {cmd}", flush=True)

    # 1. Respect user's preferred terminal via modern FreeDesktop standard (Ubuntu 24+, Fedora, Arch)
    if shutil.which("xdg-terminal-exec"):
        try:
            print("Launching via xdg-terminal-exec", flush=True)
            subprocess.Popen(["xdg-terminal-exec", "--title=RcloneDash", "--", cmd], start_new_session=True)
            return True
        except Exception as e:
            print(f"xdg-terminal-exec failed: {e}", flush=True)

    # 2. Try desktop launcher via gio launch or gtk-launch (supports ~/.local and /usr/share)
    desktop_file = os.path.expanduser("~/.local/share/applications/rclonedash.desktop")
    if not os.path.isfile(desktop_file):
        desktop_file = "/usr/share/applications/rclonedash.desktop"

    if shutil.which("gio") and os.path.isfile(desktop_file):
        try:
            print(f"Launching via gio launch {desktop_file}", flush=True)
            subprocess.Popen(["gio", "launch", desktop_file], start_new_session=True)
            return True
        except Exception as e:
            print(f"gio launch failed: {e}", flush=True)

    if shutil.which("gtk-launch"):
        try:
            print("Launching via gtk-launch rclonedash", flush=True)
            subprocess.Popen(["gtk-launch", "rclonedash"], start_new_session=True)
            return True
        except Exception as e:
            print(f"gtk-launch failed: {e}", flush=True)

    # 3. Direct terminal emulator fallback with verified flags
    terminals = [
        ["ptyxis", "--new-window", "-T", "RcloneDash", "--", cmd],
        ["gnome-terminal", "--title=RcloneDash", "--", cmd],
        ["konsole", "-p", "tabtitle=RcloneDash", "-e", cmd],
        ["alacritty", "--title", "RcloneDash", "--command", cmd],
        ["kitty", "--title", "RcloneDash", cmd],
        ["wezterm", "start", "--", cmd],
        ["ghostty", "-e", cmd],
        ["xfce4-terminal", "--title=RcloneDash", "-e", cmd],
        ["foot", "-T", "RcloneDash", cmd],
        ["x-terminal-emulator", "-e", cmd],
        ["xterm", "-title", "RcloneDash", "-e", cmd],
    ]

    for term in terminals:
        if shutil.which(term[0]):
            try:
                print(f"Launching terminal: {' '.join(term)}", flush=True)
                subprocess.Popen(term, start_new_session=True)
                return True
            except Exception as e:
                print(f"Failed to launch {term[0]}: {e}", flush=True)
                continue

    return False

def main():
    exit_code = str(sys.argv[1]) if len(sys.argv) > 1 else "1"
    reason = RCLONE_EXIT_REASONS.get(exit_code)
    summary = "Sync Failed"
    if reason:
        body = f"Bisync failed ({reason}, code {exit_code}). Click to inspect."
    else:
        body = f"Bisync exited with code {exit_code}. Click to inspect."

    try:
        import gi
        gi.require_version('Notify', '0.7')
        from gi.repository import Notify, GLib

        Notify.init("RcloneDash")
        try:
            loop = GLib.MainLoop()

            def on_action(notification, action, user_data):
                print(f"Action invoked: {action}", flush=True)
                open_rclonedash()
                loop.quit()

            def on_closed(notification):
                print("Notification closed by user or system", flush=True)
                loop.quit()

            n = Notify.Notification.new(
                summary,
                body,
                "rclonedash"
            )
            n.set_urgency(Notify.Urgency.CRITICAL)
            n.set_hint("desktop-entry", GLib.Variant("s", "rclonedash"))
            n.add_action("default", "Open", on_action, None)
            n.add_action("open", "View Dashboard", on_action, None)
            n.connect("closed", on_closed)
            n.show()

            # Automatically quit after 5 minutes if no user interaction
            GLib.timeout_add_seconds(300, loop.quit)
            loop.run()
        finally:
            try:
                if Notify.is_initted():
                    Notify.uninit()
            except Exception:
                pass

    except Exception:
        # Fallback to simple notify-send if libnotify/gi is missing
        if shutil.which("notify-send"):
            try:
                subprocess.run([
                    "notify-send",
                    summary,
                    body,
                    "--icon=rclonedash",
                    "-u", "critical",
                    "-a", "RcloneDash"
                ], check=False)
            except Exception:
                pass

if __name__ == "__main__":
    main()

