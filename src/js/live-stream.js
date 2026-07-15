import { bus } from './state.js';

/* ═══════════════════════════════════════════════════
   FLUX LIVE (WebSocket)
   Le bloc « sync en cours » est mis à jour en temps réel
   via WebSocket (~1 s). Le socket se reconnecte tout seul
   en cas de coupure (exponential backoff).
   ═══════════════════════════════════════════════════ */
export function initLiveStream() {
  if (typeof WebSocket === 'undefined') return;

  let ws;
  let retryDelay = 1000;

  function connect() {
    let protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    let url = protocol + '//' + location.host + '/api/ws';
    ws = new WebSocket(url);

    ws.onmessage = function (e) {
      try {
        let d = JSON.parse(e.data);
        // On émet l'événement sur le bus au lieu d'appeler directement la fonction
        bus.emit('live:update', d.live);
      } catch (_) {
        /* trame partielle : on ignore */
      }
    };

    ws.onopen = function () {
      retryDelay = 1000; // reset
    };

    ws.onclose = function () {
      setTimeout(connect, retryDelay);
      retryDelay = Math.min(retryDelay * 2, 30000); // max 30s
    };
  }

  connect();
}
