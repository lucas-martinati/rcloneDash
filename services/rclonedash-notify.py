#!/usr/bin/env python3
"""
RcloneDash failure notification with interactive click-to-open TUI.
"""
import os
import sys
import shutil
import subprocess

def get_rclonedash_bin():
    # Check PATH first
    b = shutil.which("rclonedash")
    if b:
        return b
    # Check ~/.local/bin
    local_b = os.path.expanduser("~/.local/bin/rclonedash")
    if os.path.isfile(local_b) and os.access(local_b, os.X_OK):
        return local_b
    # Check ~/.cargo/bin
    cargo_b = os.path.expanduser("~/.cargo/bin/rclonedash")
    if os.path.isfile(cargo_b) and os.access(cargo_b, os.X_OK):
        return cargo_b
    return "rclonedash"

def open_rclonedash():
    cmd = get_rclonedash_bin()
    print(f"open_rclonedash using binary: {cmd}", flush=True)

    # Prioritize popular terminal emulators for direct, reliable launch
    terminals = [
        ["ptyxis", "--new-window", "--", cmd],
        ["gnome-terminal", "--", cmd],
        ["konsole", "-e", cmd],
        ["alacritty", "-e", cmd],
        ["kitty", cmd],
        ["xfce4-terminal", "-e", cmd],
        ["foot", cmd],
        ["x-terminal-emulator", "-e", cmd],
        ["xterm", "-e", cmd],
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

    # Fallback to desktop launcher if gtk-launch is available
    desktop_file = os.path.expanduser("~/.local/share/applications/rclonedash.desktop")
    if shutil.which("gtk-launch") and os.path.isfile(desktop_file):
        try:
            print("Launching via gtk-launch rclonedash", flush=True)
            subprocess.Popen(["gtk-launch", "rclonedash"], start_new_session=True)
            return True
        except Exception as e:
            print(f"gtk-launch failed: {e}", flush=True)

    return False

def main():
    exit_code = sys.argv[1] if len(sys.argv) > 1 else "1"
    msg = f"La dernière synchronisation a échoué (code {exit_code}). Cliquez pour ouvrir le tableau de bord."

    try:
        import gi
        gi.require_version('Notify', '0.7')
        from gi.repository import Notify, GLib

        Notify.init("RcloneDash")
        loop = GLib.MainLoop()

        def on_action(notification, action, user_data):
            print(f"Action invoked: {action}", flush=True)
            open_rclonedash()
            loop.quit()

        def on_closed(notification):
            print("Notification closed by user or system", flush=True)
            loop.quit()

        n = Notify.Notification.new(
            "RcloneDash — Échec de synchronisation",
            msg,
            "dialog-error"
        )
        n.set_urgency(Notify.Urgency.CRITICAL)
        n.add_action("default", "Ouvrir", on_action, None)
        n.add_action("open", "Ouvrir RcloneDash", on_action, None)
        n.connect("closed", on_closed)
        n.show()

        # Automatically quit after 5 minutes if no user interaction
        GLib.timeout_add_seconds(300, loop.quit)
        loop.run()

    except Exception:
        # Fallback to simple notify-send if libnotify/gi is missing
        if shutil.which("notify-send"):
            try:
                subprocess.run([
                    "notify-send",
                    "RcloneDash — Échec de synchronisation",
                    msg,
                    "--icon=dialog-error",
                    "-u", "critical",
                    "-a", "RcloneDash"
                ], check=False)
            except Exception:
                pass

if __name__ == "__main__":
    main()
