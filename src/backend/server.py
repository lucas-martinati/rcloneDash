import os
import threading
import socketserver
from . import config
from .monitor import Monitor
from .handler import Handler

class ThreadingServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    daemon_threads = True
    allow_reuse_address = True

def main():
    Handler._m = Monitor()
    Handler._d = os.path.dirname(os.path.abspath(__file__))
    Handler._d = os.path.dirname(Handler._d) # Go up one level to src

    threading.Thread(target=Handler._m.update_quota, daemon=True).start()
    print(f"\033[1;36mRcloneDash\033[0m — Démarrage du serveur sur le port {config.PORT}")
    try:
        with ThreadingServer(("", config.PORT), Handler) as httpd:
            httpd.serve_forever()
    except KeyboardInterrupt:
        pass
