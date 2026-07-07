/* ═══════════════════════════════════════════════════
   REFRESH PRINCIPAL
   ═══════════════════════════════════════════════════ */
async function refresh() {
  spin(true);
  try {
    var r = await fetch('/api/status');
    if (!r.ok) throw new Error(r.status);
    var d = await r.json();

    badge(d.service.state);
    setSmartRefresh(d.service.state);
    updatePulse(d);
    updateAlerts(d);
    updateKPIs(d);
    updateLive(d.live);
    updateRuns(d.runs);
    updateLogs(d.logs);
    updateRecentFiles(d.recent_files);
    document.getElementById('ts').textContent = 'MàJ ' + fmtT(d.ts);
  } catch (e) {
    document.getElementById('ts').textContent = '⚠ serveur injoignable';
  } finally {
    spin(false);
  }
}

/* ═══════════════════════════════════════════════════
   ACTIONS SYNC
   ═══════════════════════════════════════════════════ */
async function doSync() {
  var b = document.getElementById('bsync');
  var lbl = document.getElementById('bsync-lbl');
  b.disabled = true;
  lbl.textContent = 'Démarrage…';
  try {
    var r = await fetch('/api/trigger');
    var d = await r.json();
    if (d.ok) {
      toast('Synchronisation lancée', 'ok');
    } else {
      toast('Impossible de lancer la synchronisation : ' + (d.error || 'erreur inconnue'), 'err');
    }
  } catch {
    toast('Serveur injoignable — synchronisation non lancée', 'err');
  }
  setTimeout(function () {
    b.disabled = false;
    lbl.textContent = 'Synchroniser';
  }, 3000);
  setTimeout(refresh, 1500);
}

async function cancelSync() {
  var b = document.getElementById('bcancel');
  var lbl = document.getElementById('bcancel-lbl');
  b.disabled = true;
  lbl.textContent = 'Arrêt…';
  try {
    await fetch('/api/cancel');
    toast('Arrêt de la synchronisation demandé', 'warn');
  } catch (e) {
    toast('Serveur injoignable', 'err');
  }
  setTimeout(function () {
    b.disabled = false;
    lbl.textContent = 'Arrêter';
  }, 3000);
  setTimeout(refresh, 1000);
}
