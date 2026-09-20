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
        ["ptyxis", "--title=RcloneDash", "--new-window", "--", cmd],
        ["gnome-terminal", "--title=RcloneDash", "--", cmd],
        ["konsole", "--title", "RcloneDash", "-e", cmd],
        ["alacritty", "--title", "RcloneDash", "-e", cmd],
        ["kitty", "--title", "RcloneDash", cmd],
        ["xfce4-terminal", "--title=RcloneDash", "-e", cmd],
        ["foot", "--title=RcloneDash", cmd],
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
    summary = "Sync Failed"
    body = f"Bisync exited with code {exit_code}. Click to inspect."

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
