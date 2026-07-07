import { updateLive } from './live-sync.js';

/* ═══════════════════════════════════════════════════
   FLUX LIVE (SSE)
   Le bloc « sync en cours » est mis à jour en temps réel
   via Server-Sent Events (~1 s), au lieu d'attendre le
   poll global de 10 s. EventSource se reconnecte tout seul
   en cas de coupure ; le poll de /api/status reste le
   filet de sécurité (il ne touche plus au live).
   ═══════════════════════════════════════════════════ */
export function initLiveStream() {
  if (typeof EventSource === 'undefined') return; // fallback : le poll gère
  let es = new EventSource('/api/live/stream');
  es.onmessage = function (e) {
    try {
      let d = JSON.parse(e.data);
      updateLive(d.live);
    } catch (_) { /* trame partielle : on ignore */ }
  };
  // En cas d'erreur, EventSource retente automatiquement — rien à faire.
}
