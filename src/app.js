(() => {
  // src/js/state.js
  var S = {
    theme: document.documentElement.dataset.theme || "dark",
    llc: 0,
    interval: null,
    curState: "",
    logFilter: "all",
    lastLogs: "",
    nextSyncTs: 0,
    // timestamp (ms) de la prochaine sync planifiée
    lastStartTs: 0,
    // timestamp (ms) du dernier déclenchement
    isSyncing: false,
    livePct: -1,
    // % de transfert connu pendant une sync
    runsSig: "",
    // signatures des dernières données rendues,
    recentSig: ""
    // pour ne pas reconstruire le DOM inutilement
  };
  var bus = {
    events: {},
    on(event, listener) {
      if (!this.events[event]) this.events[event] = [];
      this.events[event].push(listener);
    },
    emit(event, data) {
      if (this.events[event]) {
        this.events[event].forEach((l) => l(data));
      }
    }
  };

  // src/js/theme.js
  function applyThemeIcon() {
    document.getElementById("ti").innerHTML = S.theme === "dark" ? '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>' : '<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>';
  }
  function toggleTheme() {
    S.theme = S.theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = S.theme;
    localStorage.setItem("dash_theme", S.theme);
    applyThemeIcon();
  }

  // src/js/drag-resize.js
  var dragSrcEl = null;
  function handleDragStart(e) {
    dragSrcEl = this;
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/html", this.innerHTML);
    this.classList.add("dragging");
  }
  function handleDragOver(e) {
    if (e.preventDefault) {
      e.preventDefault();
    }
    e.dataTransfer.dropEffect = "move";
    return false;
  }
  function handleDragEnter() {
    this.classList.add("over");
  }
  function handleDragLeave() {
    this.classList.remove("over");
  }
  function handleDrop(e) {
    if (e.stopPropagation) {
      e.stopPropagation();
    }
    if (dragSrcEl !== this) {
      let srcOrder = window.getComputedStyle(dragSrcEl).order;
      let destOrder = window.getComputedStyle(this).order;
      if (srcOrder === destOrder) {
        let panels = document.querySelectorAll(".drag-panel");
        panels.forEach(function(p, i) {
          p.style.order = p.style.order || i;
        });
        srcOrder = dragSrcEl.style.order;
        destOrder = this.style.order;
      }
      dragSrcEl.style.order = destOrder;
      this.style.order = srcOrder;
      let orderData = {};
      document.querySelectorAll(".drag-panel").forEach(function(p) {
        p.style.height = "";
        orderData[p.id] = p.style.order;
      });
      localStorage.setItem("dash_panel_order", JSON.stringify(orderData));
      updateFullWidthPanel();
    }
    return false;
  }
  function handleDragEnd() {
    this.classList.remove("dragging");
    document.querySelectorAll(".drag-panel").forEach(function(p) {
      p.classList.remove("over");
    });
  }
  function initDragAndDrop() {
    let panels = document.querySelectorAll(".drag-panel");
    let savedOrder = JSON.parse(localStorage.getItem("dash_panel_order") || "{}");
    panels.forEach(function(panel, idx) {
      panel.style.order = savedOrder[panel.id] || idx;
      let header = panel.querySelector(".ph");
      if (header) {
        header.addEventListener("mouseenter", function() {
          panel.setAttribute("draggable", "true");
        });
        header.addEventListener("mouseleave", function() {
          panel.removeAttribute("draggable");
        });
      }
      panel.addEventListener("dragstart", handleDragStart, false);
      panel.addEventListener("dragenter", handleDragEnter, false);
      panel.addEventListener("dragover", handleDragOver, false);
      panel.addEventListener("dragleave", handleDragLeave, false);
      panel.addEventListener("drop", handleDrop, false);
      panel.addEventListener("dragend", handleDragEnd, false);
    });
    updateFullWidthPanel();
    initResizer();
  }
  function updateFullWidthPanel() {
    let panels = Array.from(document.querySelectorAll(".drag-panel"));
    panels.sort(function(a, b) {
      return parseInt(a.style.order || 0) - parseInt(b.style.order || 0);
    });
    panels.forEach(function(p, idx) {
      p.classList.toggle("full-width", idx === 2);
    });
  }
  function initResizer() {
    let container = document.getElementById("modules-container");
    let isResizingH = false;
    let isResizingV = false;
    let startX, startY, startCol1, startCol2, startRow1, startRow2;
    let savedCol = localStorage.getItem("dash_col_ratio");
    if (savedCol) {
      let parts = savedCol.split(":");
      container.style.setProperty("--col1", parts[0] + "fr");
      container.style.setProperty("--col2", parts[1] + "fr");
    }
    let savedRow = localStorage.getItem("dash_row_sizes");
    if (savedRow) {
      let rParts = savedRow.split(":");
      container.style.setProperty("--row1", rParts[0] + "px");
      container.style.setProperty("--row2", rParts[1] + "px");
    }
    let hoverRaf = 0;
    let lastMx = 0, lastMy = 0;
    function updateHoverCursor(clientX, clientY) {
      let panels = Array.from(document.querySelectorAll(".drag-panel"));
      panels.sort(function(a, b) {
        return parseInt(a.style.order || 0) - parseInt(b.style.order || 0);
      });
      if (panels.length < 3) return;
      let p1 = panels[0].getBoundingClientRect();
      let p2 = panels[1].getBoundingClientRect();
      let p3 = panels[2].getBoundingClientRect();
      let isH = clientX > p1.right - 5 && clientX < p2.left + 5 && clientY > p1.top && clientY < p1.bottom;
      let isV = clientY > p1.bottom - 5 && clientY < p3.top + 5 && clientX > p3.left && clientX < p3.right;
      let cursor = isH && isV ? "move" : isH ? "col-resize" : isV ? "row-resize" : "";
      if (container.style.cursor !== cursor) container.style.cursor = cursor;
    }
    container.addEventListener("mousemove", function(e) {
      if (isResizingH) {
        let rect = container.getBoundingClientRect();
        let dx = e.clientX - startX;
        let c1 = startCol1 + dx / rect.width * (startCol1 + startCol2);
        let c2 = startCol2 - dx / rect.width * (startCol1 + startCol2);
        if (c1 > 0.1 && c2 > 0.1) {
          container.style.setProperty("--col1", c1 + "fr");
          container.style.setProperty("--col2", c2 + "fr");
          localStorage.setItem("dash_col_ratio", c1 + ":" + c2);
        }
        return;
      }
      if (isResizingV) {
        let dy = e.clientY - startY;
        let r1 = startRow1 + dy;
        let r2 = startRow2 - dy;
        if (r1 > 100 && r2 > 100) {
          container.style.setProperty("--row1", r1 + "px");
          container.style.setProperty("--row2", r2 + "px");
          localStorage.setItem("dash_row_sizes", r1 + ":" + r2);
        }
        return;
      }
      lastMx = e.clientX;
      lastMy = e.clientY;
      if (hoverRaf) return;
      hoverRaf = requestAnimationFrame(function() {
        hoverRaf = 0;
        updateHoverCursor(lastMx, lastMy);
      });
    });
    container.addEventListener("mousedown", function(e) {
      if (container.style.cursor === "col-resize" || container.style.cursor === "move") {
        isResizingH = true;
        startX = e.clientX;
        startCol1 = parseFloat(getComputedStyle(container).getPropertyValue("--col1")) || 1;
        startCol2 = parseFloat(getComputedStyle(container).getPropertyValue("--col2")) || 1;
        e.preventDefault();
      }
      if (container.style.cursor === "row-resize" || container.style.cursor === "move") {
        isResizingV = true;
        startY = e.clientY;
        startRow1 = parseFloat(getComputedStyle(container).getPropertyValue("--row1")) || 300;
        startRow2 = parseFloat(getComputedStyle(container).getPropertyValue("--row2")) || 260;
        e.preventDefault();
      }
      if (isResizingH || isResizingV) document.body.style.cursor = container.style.cursor;
    });
    window.addEventListener("mouseup", function() {
      isResizingH = false;
      isResizingV = false;
      document.body.style.cursor = "";
    });
  }

  // src/js/utils.js
  function esc(s) {
    return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }
  function fmtT(ts) {
    if (!ts) return "\u2014";
    try {
      return new Date(ts).toLocaleTimeString("fr-FR", {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit"
      });
    } catch {
      return ts.slice(11, 19) || ts;
    }
  }
  function fmtDT(ts) {
    if (!ts) return "\u2014";
    let d = new Date(ts);
    if (isNaN(d)) return ts.slice(0, 16);
    let now = /* @__PURE__ */ new Date();
    let yest = new Date(now);
    yest.setDate(now.getDate() - 1);
    let hm = d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
    if (d.toDateString() === now.toDateString()) return "Auj. " + hm;
    if (d.toDateString() === yest.toDateString()) return "Hier " + hm;
    return d.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit" }) + " " + hm;
  }
  function spin(v) {
    document.getElementById("spin").classList.toggle("on", v);
  }
  function fmtSize(bytes) {
    if (bytes == null || isNaN(bytes)) return "";
    if (bytes < 1024) return bytes + " o";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " Ko";
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " Mo";
    if (bytes < 1024 ** 4) return (bytes / 1024 ** 3).toFixed(1) + " Go";
    return (bytes / 1024 ** 4).toFixed(2) + " To";
  }
  function parseElapsed(e) {
    if (!e) return 0;
    let s = 0;
    let m = e.match(/(\d+)h/);
    if (m) s += parseInt(m[1]) * 3600;
    m = e.match(/(\d+)m(?!s)/);
    if (m) s += parseInt(m[1]) * 60;
    m = e.match(/([\d.]+)s/);
    if (m) s += parseFloat(m[1]);
    return s;
  }
  function fmtRemaining(s) {
    if (s <= 0) return "imminente\u2026";
    if (s < 60) return Math.round(s) + " s";
    if (s < 3600)
      return Math.floor(s / 60) + " min " + String(Math.round(s % 60)).padStart(2, "0") + " s";
    return Math.floor(s / 3600) + " h " + String(Math.floor(s % 3600 / 60)).padStart(2, "0") + " min";
  }
  function colorizeLog(text) {
    let e = esc(text);
    e = e.replace(
      /^\d{4}[-/]\d{2}[-/]\d{2}[T ](\d{2}:\d{2}:\d{2})[^\s]*\s+(\S+)\s+(\S+?):\s*/,
      '<span class="log-meta">$1</span> '
    );
    if (e.indexOf("log-meta") === -1) {
      e = e.replace(
        /^(\d{4}[-/]\d{2}[-/]\d{2}[T ](\d{2}:\d{2}:\d{2}))[^\s]*\s*/,
        '<span class="log-meta">$2</span> '
      );
    }
    e = e.replace(/INFO\+?\d*(:| )\s*/g, '<strong style="color:var(--faint)">INFO : </strong>');
    e = e.replace(/ERROR(:| )\s*/g, '<strong style="color:var(--err)">ERREUR : </strong>');
    e = e.replace(/NOTICE(:| )\s*/g, '<strong style="color:var(--warn)">NOTICE : </strong>');
    e = e.replace(/DEBUG(:| )\s*/g, '<strong style="color:var(--faint)">DEBUG : </strong>');
    e = e.replace(/(Deleted .*|File was deleted.*)/g, '<span style="color:var(--err)">$1</span>');
    e = e.replace(/(Copied .*|File is new.*)/g, '<span style="color:var(--ok)">$1</span>');
    e = e.replace(/(Updated .*|File was modified.*)/g, '<span style="color:var(--warn)">$1</span>');
    return e;
  }
  function splitPath(path) {
    let full = String(path);
    let slash = full.lastIndexOf("/");
    return {
      name: slash >= 0 ? full.slice(slash + 1) : full,
      dir: slash >= 0 ? full.slice(0, slash) : ""
    };
  }
  function renderPath(path, isDir) {
    let p = splitPath(path);
    return '<span class="recent-path" title="' + esc(String(path)) + '"><span class="rp-name' + (isDir ? " is-dir" : "") + '">' + esc(p.name) + '</span><span class="rp-dir' + (p.dir ? "" : " root") + '">' + esc(p.dir || "Racine") + "</span></span>";
  }
  function renderFileRow(f, extraStyle, opts) {
    opts = opts || {};
    let cls = f.action || "new";
    let sizeTxt = opts.customSize || fmtSize(f.size);
    let labels = { new: "Copi\xE9", modified: "Modifi\xE9", deleted: "Supprim\xE9", excluded: "Exclu" };
    let timeTxt = "";
    if (f.time && !opts.hideTime) {
      let t = String(f.time);
      timeTxt = /\d{4}-\d{2}-\d{2}/.test(t) ? fmtDT(t) : t.indexOf(":") !== -1 ? esc(t) : fmtT(t);
    }
    let styleAttr = extraStyle ? ' style="' + extraStyle + '"' : "";
    let openAttrs = ' data-openfile="1" data-fpath="' + esc(f.path) + '" data-deleted="' + (cls === "deleted" ? "1" : "0") + '"';
    let itemCls = "recent-item file-link" + (opts.hideAction ? "" : " " + cls);
    let html = '<div class="' + itemCls + '"' + openAttrs + ' title="Ouvrir le fichier \u2014 Ctrl+clic pour ouvrir son dossier"' + styleAttr + ">";
    if (!opts.hideAction) {
      html += '<span class="recent-label ' + cls + '">' + (labels[cls] || cls) + "</span>";
    }
    html += renderPath(f.path, f.is_dir);
    if (sizeTxt) html += '<span class="recent-size">' + sizeTxt + "</span>";
    if (timeTxt) html += '<span class="recent-time">' + timeTxt + "</span>";
    html += "</div>";
    return html;
  }

  // src/js/pulse.js
  function updatePulse(d) {
    let runs = d.runs || [];
    let last = null;
    for (let i = 0; i < runs.length; i++) {
      if (runs[i].status !== "running") {
        last = runs[i];
        break;
      }
    }
    let dot = document.getElementById("pulse-dot");
    let lastEl = document.getElementById("pulse-last");
    S.isSyncing = d.service.state === "running" || d.live && d.live.is_syncing;
    if (S.isSyncing) {
      dot.className = "pulse-dot run";
    } else if (last) {
      dot.className = "pulse-dot " + (last.status === "success" ? "ok" : "err");
    } else {
      dot.className = "pulse-dot";
    }
    if (last) {
      let mark = last.status === "success" ? '<span class="st-ok">\u2713</span>' : '<span class="st-err">\u2717</span>';
      lastEl.innerHTML = esc(fmtT(last.start)) + " " + mark + (last.elapsed ? ' <span style="color:var(--muted)">\xB7 ' + esc(last.elapsed) + "</span>" : "");
    } else {
      lastEl.textContent = "\u2014";
    }
    let mid = document.getElementById("pulse-mid");
    if (S.isSyncing && d.live && d.live.phase) {
      mid.style.display = "";
      mid.textContent = d.live.phase;
    } else {
      mid.style.display = "none";
    }
    let cap = document.getElementById("pulse-next-cap");
    let next = document.getElementById("pulse-next");
    let raw = d.timer && d.timer.next_run || "";
    let m = raw.match(/(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})/);
    if (d.timer && !d.timer.active) {
      S.nextSyncTs = null;
      cap.textContent = "Planification";
      next.textContent = "Timer inactif";
      next.style.color = "var(--warn)";
    } else if (!m && S.isSyncing) {
      S.nextSyncTs = null;
      cap.textContent = "Prochaine sync";
      next.textContent = "apr\xE8s celle-ci";
      next.style.color = "";
    } else if (m) {
      S.nextSyncTs = (/* @__PURE__ */ new Date(m[1] + "T" + m[2])).getTime();
      cap.textContent = "Prochaine sync";
      next.style.color = "";
    } else {
      S.nextSyncTs = null;
      cap.textContent = "Prochaine sync";
      next.textContent = raw && raw !== "\u2014" ? raw : "\u2014";
      next.style.color = "";
    }
    if (runs.length) S.lastStartTs = new Date(runs[0].start).getTime() || null;
    S.livePct = S.isSyncing && d.live && d.live.transfer && d.live.transfer.pct != null ? d.live.transfer.pct : null;
    document.getElementById("pulse").classList.toggle("syncing", S.isSyncing);
    tickPulse();
  }
  function tickPulse() {
    let next = document.getElementById("pulse-next");
    let line = document.getElementById("pulse-line");
    if (S.isSyncing) {
      line.style.transform = "";
      if (S.livePct != null && S.livePct > 0) {
        line.classList.remove("indet");
        line.style.width = S.livePct + "%";
      } else {
        line.classList.add("indet");
      }
    } else {
      line.classList.remove("indet");
    }
    if (S.nextSyncTs) {
      let remS = (S.nextSyncTs - Date.now()) / 1e3;
      next.textContent = "dans " + fmtRemaining(remS);
      if (!S.isSyncing) {
        let cycleS = 600;
        if (S.lastStartTs && S.nextSyncTs > S.lastStartTs) {
          cycleS = (S.nextSyncTs - S.lastStartTs) / 1e3;
        }
        let pct = Math.min(100, Math.max(0, (1 - remS / cycleS) * 100));
        line.style.width = pct + "%";
      }
    } else if (!S.isSyncing) {
      line.style.width = "0";
    }
  }

  // src/js/dashboard.js
  function updateQuota(q) {
    let txt = document.getElementById("quota-text");
    let sub = document.getElementById("quota-sub");
    let bar = document.getElementById("quota-bar");
    if (!q) return;
    if (q.error) {
      txt.textContent = "Erreur";
      txt.style.color = "var(--err)";
      sub.textContent = "quota Google Drive indisponible";
      sub.title = q.error;
      return;
    }
    let used = q.used || 0;
    let total = q.total || 1;
    let pct = Math.min(100, Math.round(used / total * 100));
    txt.textContent = fmtSize(used);
    txt.style.color = "";
    sub.textContent = "sur " + fmtSize(total) + " \u2014 " + pct + " %";
    sub.title = "";
    bar.style.width = Math.max(pct, 0.5) + "%";
    bar.classList.toggle("danger", pct > 90);
  }
  function updateAlerts(data) {
    updateQuota(data.quota);
    let ban = document.getElementById("alert-banner");
    let msg = document.getElementById("alert-msg");
    let kpis = data.kpis;
    let disk = data.disk;
    let live = data.live;
    if (kpis.consecutive_failures >= 2) {
      ban.className = "alert-banner show-err";
      msg.textContent = kpis.consecutive_failures + " syncs cons\xE9cutives en erreur \u2014 " + (kpis.last_error_msg || "consultez les logs pour le d\xE9tail");
    } else if (live && live.is_syncing && live.duration_s > 300) {
      ban.className = "alert-banner show-warn";
      msg.textContent = "Synchronisation longue \u2014 en cours depuis " + Math.floor(live.duration_s / 60) + " min " + live.duration_s % 60 + " s";
    } else if (disk.pct > 90) {
      ban.className = "alert-banner show-err";
      msg.textContent = "Disque local rempli \xE0 " + disk.pct + " % \u2014 lib\xE9rez de l'espace";
    } else {
      ban.className = "alert-banner";
    }
    let sb = document.getElementById("slow-badge");
    sb.style.display = live && live.is_syncing && live.duration_s > 300 ? "" : "none";
  }
  function updateKPIs(data) {
    let disk = data.disk;
    let runs = data.runs || [], kpis = data.kpis;
    let dkpi = document.getElementById("disk-kpi");
    let dkEl = document.getElementById("kdk");
    dkEl.textContent = disk.used + " Go";
    document.getElementById("kdks").textContent = disk.free + " Go libres sur " + disk.total + " Go";
    document.getElementById("dfill").style.width = disk.pct + "%";
    let diskDanger = disk.pct > 90;
    document.getElementById("dfill").classList.toggle("danger", diskDanger);
    dkpi.classList.toggle("danger", diskDanger);
    dkEl.style.color = diskDanger ? "var(--err)" : "";
    let today = (/* @__PURE__ */ new Date()).toISOString().slice(0, 10);
    let td = runs.filter(function(r) {
      return r.start && r.start.startsWith(today);
    });
    let tok = td.filter(function(r) {
      return r.status === "success";
    }).length;
    let terr = td.filter(function(r) {
      return r.status === "failed";
    }).length;
    document.getElementById("kr").textContent = td.length;
    document.getElementById("krs").textContent = tok + " r\xE9ussie(s)" + (terr ? " \xB7 " + terr + " en erreur" : "");
    document.getElementById("kf").textContent = kpis.total_files > 0 ? kpis.total_files.toLocaleString("fr-FR") : "\u2014";
    document.getElementById("ksp").textContent = kpis.avg_speed || "\u2014";
    let kcEl = document.getElementById("kc");
    kcEl.textContent = kpis.conflicts_today;
    kcEl.style.color = kpis.conflicts_today > 0 ? "var(--warn)" : "";
    document.getElementById("conflict-kpi").classList.toggle("danger", kpis.conflicts_today > 0);
    let rateVal = kpis.success_rate_7d;
    document.getElementById("ksr").textContent = rateVal + " %";
    document.getElementById("ksr").style.color = rateVal < 90 ? "var(--err)" : rateVal < 99 ? "var(--warn)" : "";
    document.getElementById("srfill").style.width = rateVal + "%";
    document.getElementById("srfill").classList.toggle("danger", rateVal < 90);
  }

  // src/js/sparkline.js
  window._sparkTips = [];
  function renderSparkline(runs) {
    let wrap = document.getElementById("sparkline-wrap");
    let data = runs.slice(0, 24).reverse();
    if (data.length < 2) {
      wrap.innerHTML = "";
      return;
    }
    let w = wrap.clientWidth || 300, h = 54, px = 6, py = 6;
    let values = data.map(function(r) {
      return parseElapsed(r.elapsed);
    });
    let maxV = Math.max.apply(null, values);
    if (maxV === 0) {
      values = data.map(function(r) {
        return r.files || 0;
      });
      maxV = Math.max.apply(null, values) || 1;
    }
    window._sparkTips = [];
    let slot = (w - px * 2) / data.length;
    let html = '<svg viewBox="0 0 ' + w + " " + h + '" style="width:100%;height:' + h + 'px;display:block" preserveAspectRatio="none" aria-label="Dur\xE9e des derni\xE8res syncs">';
    for (let i = 0; i < data.length; i++) {
      let r = data[i];
      let v = values[i];
      let bw = Math.max(3, slot - 3);
      let bx = px + i * slot;
      let bh = Math.max(2, v / maxV * (h - py * 2));
      let by = h - py - bh;
      let color = r.status === "failed" ? "var(--err)" : r.status === "success" ? "var(--ok)" : "var(--run)";
      let st = r.status === "failed" ? "\u2717 erreur" : r.status === "success" ? "\u2713 r\xE9ussie" : "\u27F3 en cours";
      window._sparkTips.push(
        fmtDT(r.start) + " \u2014 " + (r.elapsed || "\u2014") + " \xB7 " + (r.files || 0) + " fichier(s) \xB7 " + st
      );
      html += '<rect class="chart-bar" x="' + bx + '" y="' + by + '" width="' + bw + '" height="' + bh + '" fill="' + color + '" rx="1.5" onmousemove="showTooltip(event, _sparkTips[' + i + '])" onmouseout="hideTooltip()"/>';
    }
    html += "</svg>";
    wrap.innerHTML = html;
    wrap.insertAdjacentHTML("beforeend", '<div id="chart-tt" class="chart-tooltip"></div>');
  }
  function showTooltip(e, txt) {
    let tt = document.getElementById("chart-tt");
    if (!tt) return;
    tt.textContent = txt;
    tt.style.display = "block";
    let x = e.clientX + 10;
    let y = e.clientY - 25;
    if (x + tt.offsetWidth > window.innerWidth - 4) x = e.clientX - tt.offsetWidth - 10;
    if (y < 4) y = e.clientY + 15;
    tt.style.left = x + "px";
    tt.style.top = y + "px";
  }
  function hideTooltip() {
    let tt = document.getElementById("chart-tt");
    if (tt) tt.style.display = "none";
  }

  // src/js/history.js
  function updateRuns(runs) {
    let sig = JSON.stringify(runs || []);
    if (sig === S.runsSig) return;
    S.runsSig = sig;
    window._errorLogsCache = {};
    let tb = document.getElementById("rtb");
    let em = document.getElementById("rem");
    if (!runs || !runs.length) {
      tb.innerHTML = "";
      em.style.display = "";
      document.getElementById("sparkline-wrap").innerHTML = "";
      return;
    }
    em.style.display = "none";
    let openSet = {};
    tb.querySelectorAll("tr.open").forEach(function(tr) {
      openSet[tr.dataset.start] = true;
    });
    let html = "";
    for (let i = 0; i < runs.length; i++) {
      let r = runs[i];
      let statusCls = r.status === "success" ? "ok" : r.status === "failed" ? "err" : "run";
      let statusTxt = r.status === "success" ? "\u2713 R\xE9ussie" : r.status === "failed" ? "\u2717 Erreur" : "\u27F3 En cours";
      let isOpen = openSet[r.start];
      html += '<tr class="clickable' + (isOpen ? " open" : "") + '" data-start="' + esc(r.start) + '" onclick="toggleRunDetails(' + i + ')" title="Afficher le d\xE9tail de cette sync"><td style="color:var(--muted)"><span class="chev">\u25B6</span>' + fmtDT(r.start) + '</td><td><span class="pill ' + statusCls + '">' + statusTxt + '</span></td><td class="num' + (r.copied ? "" : " dim") + '">' + (r.copied || 0) + '</td><td class="num' + (r.modified ? "" : " dim") + '">' + (r.modified || 0) + '</td><td class="num' + (r.deleted ? "" : " dim") + '">' + (r.deleted || 0) + '</td><td class="num" style="color:var(--muted)">' + (r.elapsed || "\u2014") + '</td><td class="num" style="color:' + (r.errors > 0 ? "var(--err)" : "var(--faint)") + '">' + r.errors + "</td></tr>";
      let detHtml = '<div class="run-details-box">';
      detHtml += '<div class="run-details-header"><span><b>D\xE9but :</b> ' + fmtDT(r.start) + "</span><span><b>Fin :</b> " + fmtDT(r.end) + "</span><span><b>Dur\xE9e :</b> " + (r.elapsed || "\u2014") + "</span></div>";
      if (r.error_logs && r.error_logs.length > 0) {
        window._errorLogsCache[i] = r.error_logs.join("\n");
        detHtml += '<div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:4px;"><div style="color:var(--err);font-weight:600;">Erreurs :</div><button class="iconbtn" onclick="copyErrorLogs(' + i + ', this); event.stopPropagation();">Copier</button></div><div class="error-log-box">' + colorizeLog(r.error_logs.join("\n")) + "</div>";
      }
      if (r.synced_files && r.synced_files.length > 0) {
        detHtml += '<div style="font-weight:600;margin-top:10px">Fichiers affect\xE9s (' + r.synced_files.length + ") :</div>";
        detHtml += '<div class="run-details-files">';
        let labels = { new: "Copi\xE9", modified: "Modifi\xE9", deleted: "Supprim\xE9" };
        for (let j = 0; j < Math.min(r.synced_files.length, 50); j++) {
          let f = r.synced_files[j];
          let cls = f.action || "new";
          let pathArg = esc(f.path).replace(/\\/g, "\\\\").replace(/'/g, "\\'");
          let actionCall = "openFile('" + pathArg + "', " + (cls === "deleted" ? "true" : "false") + ", event)";
          let fSize = fmtSize(f.size);
          detHtml += '<div class="run-details-file recent-item file-link ' + cls + '" onclick="event.stopPropagation(); ' + actionCall + '"><span class="recent-label ' + cls + '">' + (labels[cls] || cls) + "</span>" + renderPath(f.path, f.is_dir) + (fSize ? '<span class="recent-size">' + fSize + "</span>" : "") + "</div>";
        }
        if (r.synced_files.length > 50) {
          detHtml += '<div style="padding:4px;color:var(--muted)">+ ' + (r.synced_files.length - 50) + " autres fichiers\u2026</div>";
        }
        detHtml += "</div>";
      } else {
        detHtml += `<div style="color:var(--faint);margin-top:10px">Aucun fichier n'a chang\xE9 durant cette sync.</div>`;
      }
      detHtml += "</div>";
      html += '<tr id="run-det-' + i + '" style="display:' + (isOpen ? "table-row" : "none") + '"><td colspan="7" style="padding:0 10px 8px; max-width:0; white-space:normal;">' + detHtml + "</td></tr>";
    }
    tb.innerHTML = html;
    renderSparkline(runs);
  }
  function toggleRunDetails(i) {
    let det = document.getElementById("run-det-" + i);
    if (!det) return;
    let open = det.style.display === "none";
    det.style.display = open ? "table-row" : "none";
    let row = det.previousElementSibling;
    if (row) row.classList.toggle("open", open);
  }
  function copyErrorLogs(idx, btn) {
    let text = window._errorLogsCache && window._errorLogsCache[idx];
    if (!text) return;
    navigator.clipboard.writeText(text).then(function() {
      btn.textContent = "Copi\xE9 \u2713";
      setTimeout(function() {
        btn.textContent = "Copier";
      }, 2e3);
    });
  }

  // src/js/logs.js
  function logPassesFilter(l) {
    if (S.logFilter === "all") return true;
    if (S.logFilter === "files") return l.l === "ok";
    return l.l === "error" || l.l === "warn";
  }
  function renderLogs() {
    let w = document.getElementById("lwrap");
    let sc = document.getElementById("lscroll");
    let atBot = sc.scrollHeight - sc.scrollTop - sc.clientHeight < 50;
    let html = "";
    let shown = 0;
    for (let i = 0; i < S.lastLogs.length; i++) {
      if (!logPassesFilter(S.lastLogs[i])) continue;
      html += '<div class="ll ' + S.lastLogs[i].l + '">' + colorizeLog(S.lastLogs[i].t) + "</div>";
      shown++;
    }
    if (!shown) {
      html = '<div class="empty">' + (S.logFilter === "all" ? "Les logs sont vides pour le moment." : "Aucune ligne de ce type dans les logs r\xE9cents.") + "</div>";
    }
    w.innerHTML = html;
    if (atBot) sc.scrollTop = sc.scrollHeight;
  }
  function updateLogs(logs) {
    S.lastLogs = logs || [];
    let sig = S.lastLogs.length + "|" + (S.lastLogs.length ? S.lastLogs[0].t + "|" + S.lastLogs[S.lastLogs.length - 1].t : "");
    if (sig !== S.llc) {
      S.llc = sig;
      renderLogs();
    }
  }
  function setLogFilter(f, btn) {
    S.logFilter = f;
    document.querySelectorAll(".chiprow .chip").forEach(function(c) {
      c.classList.remove("on");
    });
    btn.classList.add("on");
    renderLogs();
  }
  function logsBot() {
    let s = document.getElementById("lscroll");
    s.scrollTop = s.scrollHeight;
  }

  // src/js/recent.js
  function updateRecentFiles(files) {
    let sig = JSON.stringify(files || []);
    if (sig === S.recentSig) return;
    S.recentSig = sig;
    let list = document.getElementById("recent-list");
    let countEl = document.getElementById("recent-count");
    if (!files || !files.length) {
      list.innerHTML = '<div class="empty">Aucun fichier synchronis\xE9 r\xE9cemment.<br>Les fichiers copi\xE9s, modifi\xE9s ou supprim\xE9s appara\xEEtront ici.</div>';
      countEl.textContent = "";
      return;
    }
    countEl.textContent = "\xB7 " + files.length;
    let reversed = files.slice().reverse();
    let html = "";
    for (let i = 0; i < reversed.length; i++) {
      html += renderFileRow(reversed[i]);
    }
    list.innerHTML = html;
    filterRecent();
  }
  function filterRecent() {
    let q = document.getElementById("recent-search").value.toLowerCase();
    let items = document.querySelectorAll("#recent-list .recent-item");
    for (let i = 0; i < items.length; i++) {
      let path = (items[i].querySelector(".recent-path").getAttribute("title") || "").toLowerCase();
      items[i].style.display = path.indexOf(q) !== -1 ? "" : "none";
    }
  }

  // src/js/toasts.js
  function toast(msg, type) {
    let c = document.getElementById("toasts");
    let t = document.createElement("div");
    t.className = "toast " + (type || "");
    t.textContent = msg;
    c.appendChild(t);
    requestAnimationFrame(function() {
      t.classList.add("in");
    });
    setTimeout(function() {
      t.classList.remove("in");
      setTimeout(function() {
        t.remove();
      }, 300);
    }, 4200);
  }

  // src/js/refresh.js
  async function refresh() {
    spin(true);
    try {
      let r = await fetch("/api/status");
      if (!r.ok) throw new Error(r.status);
      let d = await r.json();
      bus.emit("sync:status", d.service.state);
      updatePulse(d);
      updateAlerts(d);
      updateKPIs(d);
      updateRuns(d.runs);
      updateLogs(d.logs);
      updateRecentFiles(d.recent_files);
      document.getElementById("ts").textContent = "M\xE0J " + fmtT(d.ts);
    } catch (e) {
      document.getElementById("ts").textContent = "\u26A0 serveur injoignable";
    } finally {
      spin(false);
    }
  }
  async function doSync() {
    let b = document.getElementById("bsync");
    let lbl = document.getElementById("bsync-lbl");
    b.disabled = true;
    lbl.textContent = "D\xE9marrage\u2026";
    try {
      let r = await fetch("/api/trigger", { method: "POST" });
      let d = await r.json();
      if (d.ok) {
        toast("Synchronisation lanc\xE9e", "ok");
      } else {
        toast("Impossible de lancer la synchronisation : " + (d.error || "erreur inconnue"), "err");
      }
    } catch {
      toast("Serveur injoignable \u2014 synchronisation non lanc\xE9e", "err");
    }
    setTimeout(function() {
      b.disabled = false;
      lbl.textContent = "Synchroniser";
    }, 3e3);
    setTimeout(refresh, 1500);
  }
  async function cancelSync() {
    let b = document.getElementById("bcancel");
    let lbl = document.getElementById("bcancel-lbl");
    b.disabled = true;
    lbl.textContent = "Arr\xEAt\u2026";
    try {
      await fetch("/api/cancel", { method: "POST" });
      toast("Arr\xEAt de la synchronisation demand\xE9", "warn");
    } catch (e) {
      toast("Serveur injoignable", "err");
    }
    setTimeout(function() {
      b.disabled = false;
      lbl.textContent = "Arr\xEAter";
    }, 3e3);
    setTimeout(refresh, 1e3);
  }

  // src/js/live-stream.js
  function initLiveStream() {
    if (typeof WebSocket === "undefined") return;
    let ws;
    let retryDelay = 1e3;
    function connect() {
      let protocol = location.protocol === "https:" ? "wss:" : "ws:";
      let url = protocol + "//" + location.host + "/api/ws";
      ws = new WebSocket(url);
      ws.onmessage = function(e) {
        try {
          let d = JSON.parse(e.data);
          bus.emit("live:update", d.live);
        } catch (_) {
          return;
        }
      };
      ws.onopen = function() {
        retryDelay = 1e3;
      };
      ws.onclose = function() {
        setTimeout(connect, retryDelay);
        retryDelay = Math.min(retryDelay * 2, 3e4);
      };
    }
    connect();
  }

  // src/js/focus-trap.js
  var SEL = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';
  function focusables(modal) {
    return Array.from(modal.querySelectorAll(SEL)).filter(function(el) {
      return !el.disabled && el.offsetParent !== null;
    });
  }
  function topModal() {
    const shown = Array.from(document.querySelectorAll(".modal-overlay.show"));
    if (!shown.length) return null;
    return shown.sort(function(a, b) {
      return (parseInt(getComputedStyle(a).zIndex, 10) || 0) - (parseInt(getComputedStyle(b).zIndex, 10) || 0);
    })[shown.length - 1];
  }
  function initFocusTrap() {
    let lastFocused = null;
    document.addEventListener("keydown", function(e) {
      if (e.key !== "Tab") return;
      const modal = topModal();
      if (!modal) return;
      const f = focusables(modal);
      if (!f.length) {
        e.preventDefault();
        return;
      }
      const first = f[0], last = f[f.length - 1];
      const active = document.activeElement;
      if (!modal.contains(active)) {
        e.preventDefault();
        first.focus();
      } else if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    });
    document.querySelectorAll(".modal-overlay").forEach(function(modal) {
      new MutationObserver(function() {
        const shown = modal.classList.contains("show");
        if (shown && !modal._trapped) {
          modal._trapped = true;
          lastFocused = document.activeElement;
          let done = false;
          const doFocus = function() {
            if (done || !modal.classList.contains("show")) return;
            done = true;
            const f = focusables(modal);
            (f[0] || modal).focus({ preventScroll: true });
          };
          modal.addEventListener("transitionend", doFocus, { once: true });
          setTimeout(doFocus, 250);
        } else if (!shown && modal._trapped) {
          modal._trapped = false;
          if (lastFocused && lastFocused.focus) lastFocused.focus({ preventScroll: true });
          lastFocused = null;
        }
      }).observe(modal, { attributes: true, attributeFilter: ["class"] });
    });
  }

  // src/js/icons.js
  function _svg(inner, sz) {
    sz = sz || 14;
    return '<svg width="' + sz + '" height="' + sz + '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' + inner + "</svg>";
  }
  var FM_ICONS = {
    folder: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>',
    image: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>',
    video: '<polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2"/>',
    audio: '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>',
    archive: '<polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/>',
    doc: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>',
    sheet: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="12" y1="11" x2="12" y2="19"/>',
    slides: '<rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>',
    pdf: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M9 13h1.5a1 1 0 0 1 0 3H9zM9 13v6"/>',
    code: '<polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>',
    file: '<path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/>'
  };
  var FM_EXT = {
    image: ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "heic", "heif", "tiff", "ico", "avif"],
    video: ["mp4", "mkv", "avi", "mov", "webm", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp"],
    audio: ["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus", "aiff"],
    archive: ["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "tgz", "zst"],
    pdf: ["pdf"],
    doc: ["doc", "docx", "odt", "rtf", "txt", "md", "pages", "tex", "epub"],
    sheet: ["xls", "xlsx", "ods", "csv", "tsv", "numbers"],
    slides: ["ppt", "pptx", "odp", "key"],
    code: [
      "js",
      "ts",
      "jsx",
      "tsx",
      "py",
      "rb",
      "go",
      "rs",
      "c",
      "cpp",
      "h",
      "hpp",
      "java",
      "php",
      "html",
      "htm",
      "css",
      "scss",
      "sass",
      "json",
      "xml",
      "yml",
      "yaml",
      "sh",
      "bash",
      "zsh",
      "sql",
      "swift",
      "kt",
      "lua",
      "vue",
      "ini",
      "toml"
    ]
  };
  function fmCategory(item) {
    if (item.is_dir) return "folder";
    let dot = item.name.lastIndexOf(".");
    if (dot < 1) return "file";
    let ext = item.name.slice(dot + 1).toLowerCase();
    for (let cat in FM_EXT) if (FM_EXT[cat].indexOf(ext) !== -1) return cat;
    return "file";
  }
  var ICO_CHECK = _svg('<path d="M20 6L9 17l-5-5"/>', 16);
  var ICO_WARN = _svg(
    '<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>',
    16
  );
  var ICO_CLOUD = _svg(
    '<path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/><polyline points="9 15 11 17 15 12"/>',
    15
  );

  // src/js/file-browser.js
  var _fm = {
    dir: "",
    // dossier courant (relatif à la racine synchronisée)
    items: [],
    // items bruts renvoyés par l'API
    view: [],
    // items filtrés + triés actuellement affichés
    sortKey: "name",
    sortAsc: true,
    filter: "",
    sel: -1,
    // index sélectionné au clavier dans `view`
    recursive: false,
    // recherche étendue aux sous-dossiers
    searchResults: [],
    // résultats renvoyés par /api/search
    searching: false,
    searchTimer: null,
    truncated: false
  };
  function openTreeModal() {
    document.getElementById("tree-modal").classList.add("show");
    loadTree("");
    setTimeout(function() {
      let s = document.getElementById("fm-search");
      if (s) s.focus();
    }, 80);
  }
  function closeTreeModal() {
    document.getElementById("tree-modal").classList.remove("show");
  }
  function treeUp() {
    if (!_fm.dir) return;
    let parts = _fm.dir.split("/");
    parts.pop();
    loadTree(parts.join("/"));
  }
  function openCurrentDir() {
    fetch("/api/open?path=" + encodeURIComponent(_fm.dir)).then(function(r) {
      return r.json();
    }).then(function(d) {
      if (!d.ok) toast("Impossible d'ouvrir ce dossier", "warn");
    });
  }
  async function loadTree(dir) {
    let list = document.getElementById("fm-list");
    list.innerHTML = fmSkeleton();
    let s = document.getElementById("fm-search");
    s.value = "";
    document.getElementById("fm-search-clear").hidden = true;
    _fm.filter = "";
    _fm.sel = -1;
    _fm.searchResults = [];
    _fm.searching = false;
    clearTimeout(_fm.searchTimer);
    try {
      let r = await fetch("/api/tree?dir=" + encodeURIComponent(dir));
      let d = await r.json();
      if (d.error) throw new Error(d.error);
      _fm.dir = d.current_dir || "";
      _fm.items = d.items || [];
      document.getElementById("tree-up-btn").disabled = !_fm.dir;
      fmRenderCrumbs();
      fmRender();
    } catch (e) {
      list.innerHTML = '<div class="fm-empty err">' + _svg(FM_ICONS.file, 22) + "<span>" + esc(e.message) + "</span></div>";
      document.getElementById("fm-footer").textContent = "";
    }
  }
  function fmRenderCrumbs() {
    let c = document.getElementById("fm-crumbs");
    let html = `<button class="fm-crumb root" onclick="loadTree('')" title="Racine synchronis\xE9e">` + _svg(
      '<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>',
      14
    ) + "</button>";
    if (_fm.dir) {
      let parts = _fm.dir.split("/");
      let acc = "";
      for (let i = 0; i < parts.length; i++) {
        acc = acc ? acc + "/" + parts[i] : parts[i];
        let last = i === parts.length - 1;
        let pathArg = acc.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
        html += '<span class="fm-crumb-sep">\u203A</span><button class="fm-crumb' + (last ? " current" : "") + `" onclick="loadTree('` + pathArg + `')">` + esc(parts[i]) + "</button>";
      }
    }
    c.innerHTML = html;
    c.scrollLeft = c.scrollWidth;
  }
  function fmSort(key) {
    if (_fm.sortKey === key) _fm.sortAsc = !_fm.sortAsc;
    else {
      _fm.sortKey = key;
      _fm.sortAsc = key === "name";
    }
    fmRender();
  }
  function fmFilter(v) {
    _fm.filter = v.trim().toLowerCase();
    document.getElementById("fm-search-clear").hidden = !v;
    _fm.sel = -1;
    if (_fm.recursive) {
      clearTimeout(_fm.searchTimer);
      if (_fm.filter.length < 2) {
        _fm.searchResults = [];
        _fm.searching = false;
        fmRender();
        return;
      }
      _fm.searching = true;
      fmRender();
      _fm.searchTimer = setTimeout(fmRunSearch, 250);
    } else {
      fmRender();
    }
  }
  function fmClearSearch() {
    let s = document.getElementById("fm-search");
    s.value = "";
    s.focus();
    fmFilter("");
  }
  function fmToggleRecursive() {
    _fm.recursive = !_fm.recursive;
    let btn = document.getElementById("fm-scope");
    btn.classList.toggle("active", _fm.recursive);
    btn.setAttribute("aria-pressed", _fm.recursive ? "true" : "false");
    _fm.searchResults = [];
    _fm.sel = -1;
    fmFilter(document.getElementById("fm-search").value);
  }
  async function fmRunSearch() {
    let q = _fm.filter;
    try {
      let r = await fetch(
        "/api/search?dir=" + encodeURIComponent(_fm.dir) + "&q=" + encodeURIComponent(q)
      );
      let d = await r.json();
      if (_fm.filter !== q || !_fm.recursive) return;
      _fm.searching = false;
      _fm.searchResults = d.items || [];
      _fm.truncated = !!d.truncated;
      fmRender();
    } catch (e) {
      _fm.searching = false;
      _fm.searchResults = [];
      fmRender();
    }
  }
  function fmRender() {
    let list = document.getElementById("fm-list");
    let recursiveActive = _fm.recursive && _fm.filter.length >= 2;
    if (recursiveActive && _fm.searching) {
      list.innerHTML = '<div class="fm-empty"><span class="fm-spin"></span><span>Recherche dans les sous-dossiers\u2026</span></div>';
      _fm.view = [];
      fmFooter();
      return;
    }
    let view;
    if (recursiveActive) {
      view = _fm.searchResults;
    } else {
      view = _fm.items;
      if (_fm.filter)
        view = view.filter(function(it) {
          return it.name.toLowerCase().indexOf(_fm.filter) !== -1;
        });
    }
    let key = _fm.sortKey, dir = _fm.sortAsc ? 1 : -1;
    view = view.slice().sort(function(a, b) {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
      let av, bv;
      if (key === "size") {
        av = a.is_dir ? -1 : a.size || 0;
        bv = b.is_dir ? -1 : b.size || 0;
      } else if (key === "mtime") {
        av = a.mtime || 0;
        bv = b.mtime || 0;
      } else {
        av = a.name.toLowerCase();
        bv = b.name.toLowerCase();
      }
      if (av < bv) return -dir;
      if (av > bv) return dir;
      return a.name.toLowerCase() < b.name.toLowerCase() ? -1 : 1;
    });
    _fm.view = view;
    document.querySelectorAll(".fm-col-btn").forEach(function(b) {
      let on = b.dataset.key === _fm.sortKey;
      b.classList.toggle("active", on);
      b.classList.toggle("asc", on && _fm.sortAsc);
      b.classList.toggle("desc", on && !_fm.sortAsc);
    });
    if (!view.length) {
      list.innerHTML = _fm.filter ? '<div class="fm-empty">' + _svg('<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>', 22) + "<span>Aucun r\xE9sultat pour \xAB " + esc(_fm.filter) + " \xBB</span></div>" : '<div class="fm-empty">' + _svg(FM_ICONS.folder, 22) + "<span>Ce dossier est vide</span></div>";
      fmFooter();
      return;
    }
    let html = "";
    for (let i = 0; i < view.length; i++) html += fmRow(view[i], i);
    list.innerHTML = html;
    fmFooter();
  }
  function fmRow(item, i) {
    let cat = fmCategory(item);
    let isDir = item.is_dir;
    let pathArg = item.path.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
    let action = isDir ? "loadTree('" + pathArg + "')" : "openFile('" + pathArg + "', false, event)";
    let meta = isDir ? item.count == null ? "\u2014" : item.count + (item.count > 1 ? " \xE9l\xE9ments" : " \xE9l\xE9ment") : fmtSize(item.size) || "\u2014";
    let modTxt = item.mtime ? fmtDT(item.mtime) : "\u2014";
    let title = isDir ? "Ouvrir le dossier" : "Ouvrir le fichier \u2014 Ctrl+clic pour ouvrir son dossier";
    let ignored = !!item.ignored;
    let revealBtn = isDir ? `<button class="fm-act" onclick="event.stopPropagation(); openFile('` + pathArg + `')" title="Ouvrir dans l'explorateur syst\xE8me">` + _svg(
      '<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>',
      13
    ) + "</button>" : `<button class="fm-act" onclick="event.stopPropagation(); openFile('` + pathArg + `', true)" title="Ouvrir le dossier contenant">` + _svg(FM_ICONS.folder, 13) + "</button>";
    let toggleBtn = ignored ? `<button class="fm-act reinc" onclick="event.stopPropagation(); reincludePath('` + pathArg + "', " + isDir + ')" title="R\xE9-inclure dans la synchronisation">' + _svg(
      '<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>',
      13
    ) + "</button>" : `<button class="fm-act" onclick="event.stopPropagation(); ignorePath('` + pathArg + "', " + isDir + ')" title="Exclure de la synchronisation">' + _svg(
      '<circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/>',
      13
    ) + "</button>";
    let delBtn = `<button class="fm-act danger" onclick="event.stopPropagation(); openDeleteModal('` + pathArg + `')" title="Supprimer localement\u2026">` + _svg(
      '<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>',
      13
    ) + "</button>";
    return '<div class="fm-row' + (i === _fm.sel ? " sel" : "") + (isDir ? " is-dir" : " file-link") + (ignored ? " ignored" : "") + '" data-i="' + i + '" onclick="' + action + '" title="' + title + '"><span class="fm-ic cat-' + cat + '">' + _svg(FM_ICONS[cat], 16) + '</span><span class="fm-name"><span class="fm-nm-txt">' + fmHighlight(item.name) + "</span>" + (ignored ? '<span class="fm-ig-badge" title="Exclu de la synchronisation">exclu</span>' : "") + '</span><span class="fm-meta col-mod" title="' + esc(modTxt) + '">' + esc(modTxt) + '</span><span class="fm-meta col-size">' + esc(meta) + '</span><span class="fm-row-actions">' + revealBtn + toggleBtn + delBtn + "</span></div>";
  }
  function fmHighlight(name) {
    if (!_fm.filter) return esc(name);
    let idx = name.toLowerCase().indexOf(_fm.filter);
    if (idx < 0) return esc(name);
    return esc(name.slice(0, idx)) + "<mark>" + esc(name.slice(idx, idx + _fm.filter.length)) + "</mark>" + esc(name.slice(idx + _fm.filter.length));
  }
  function fmFooter() {
    let f = document.getElementById("fm-footer");
    if (_fm.recursive && _fm.filter.length >= 2) {
      let n = _fm.view.length;
      let left = "Recherche dans les sous-dossiers";
      let right = (n ? n : "Aucun") + " r\xE9sultat" + (n > 1 ? "s" : "") + (_fm.truncated ? " (limit\xE9)" : "");
      f.innerHTML = "<span>" + esc(left) + '</span><span class="fm-foot-r">' + esc(right) + "</span>";
      return;
    }
    let dirs = 0, files = 0, total = 0;
    _fm.items.forEach(function(it) {
      if (it.is_dir) dirs++;
      else {
        files++;
        total += it.size || 0;
      }
    });
    let parts = [];
    if (dirs) parts.push(dirs + (dirs > 1 ? " dossiers" : " dossier"));
    if (files) parts.push(files + (files > 1 ? " fichiers" : " fichier"));
    let left2 = parts.join("  \xB7  ") || "Dossier vide";
    if (total) left2 += "  \xB7  " + fmtSize(total);
    let right2 = _fm.filter ? _fm.view.length + " r\xE9sultat" + (_fm.view.length > 1 ? "s" : "") : "";
    f.innerHTML = "<span>" + esc(left2) + "</span>" + (right2 ? '<span class="fm-foot-r">' + esc(right2) + "</span>" : "");
  }
  function fmSkeleton() {
    let row = '<div class="fm-skel"><span class="sk-ic"></span><span class="sk-l"></span><span class="sk-s"></span></div>';
    return row.repeat(8);
  }
  function fmKeydown(e) {
    if (!document.getElementById("tree-modal").classList.contains("show")) return;
    if (document.getElementById("delete-modal").classList.contains("show")) return;
    if (document.getElementById("exclude-modal").classList.contains("show")) return;
    if (document.getElementById("reinc-modal").classList.contains("show")) return;
    let searchEl = document.getElementById("fm-search");
    if (e.key === "/" && document.activeElement !== searchEl) {
      e.preventDefault();
      searchEl.focus();
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!_fm.view.length) return;
      if (_fm.sel < 0) _fm.sel = e.key === "ArrowDown" ? 0 : _fm.view.length - 1;
      else
        _fm.sel = Math.max(
          0,
          Math.min(_fm.view.length - 1, _fm.sel + (e.key === "ArrowDown" ? 1 : -1))
        );
      fmUpdateSel();
      return;
    }
    if (e.key === "Enter") {
      let idx = _fm.sel >= 0 ? _fm.sel : _fm.view.length ? 0 : -1;
      if (idx >= 0) {
        let it = _fm.view[idx];
        if (it.is_dir) loadTree(it.path);
        else openFile(it.path, false, e);
      }
      return;
    }
    if (e.key === "Backspace" && (document.activeElement !== searchEl || searchEl.value === "")) {
      e.preventDefault();
      treeUp();
    }
  }
  function fmUpdateSel() {
    document.querySelectorAll("#fm-list .fm-row").forEach(function(r) {
      let on = +r.dataset.i === _fm.sel;
      r.classList.toggle("sel", on);
      if (on) r.scrollIntoView({ block: "nearest" });
    });
  }
  window.addEventListener("keydown", fmKeydown);
  function openFile(path, isDeleted, ev) {
    let dirOnly = isDeleted || ev && (ev.ctrlKey || ev.metaKey);
    let url = "/api/open?path=" + encodeURIComponent(path) + (dirOnly ? "&dir_only=1" : "");
    fetch(url).then(function(r) {
      return r.json();
    }).then(function(d) {
      if (!d.ok)
        toast(
          dirOnly ? "Impossible d'ouvrir ce dossier" : "Impossible d'ouvrir ce fichier",
          "warn"
        );
    });
  }

  // src/js/delete.js
  var _del = { path: "" };
  function openDeleteModal(path) {
    _del = { path };
    document.getElementById("delete-modal").classList.add("show");
    document.getElementById("del-path").textContent = "/" + path;
    document.getElementById("del-summary").innerHTML = '<div class="del-loading">Analyse du contenu\u2026</div>';
    document.getElementById("del-drive").innerHTML = "";
    document.getElementById("del-files").innerHTML = "";
    document.getElementById("del-confirm").disabled = true;
    fetch("/api/delete_preview?path=" + encodeURIComponent(path)).then(function(r) {
      return r.json();
    }).then(function(d) {
      if (!d.ok) {
        document.getElementById("del-summary").innerHTML = '<div class="del-banner danger">' + ICO_WARN + "<div>Erreur : " + esc(d.error || "") + "</div></div>";
        return;
      }
      _del.ignored = d.ignored;
      let banner = d.ignored ? '<div class="del-banner ok">' + ICO_CHECK + "<div><b>Exclu de la synchronisation.</b> La suppression locale ne sera pas propag\xE9e au Drive : tu lib\xE8res seulement de l'espace sur ce PC.</div></div>" : '<div class="del-banner danger">' + ICO_WARN + "<div><b>Cet \xE9l\xE9ment n'est pas exclu de la synchronisation.</b> Le supprimer localement l'effacera aussi du Drive au prochain bisync. Exclus-le d'abord pour conserver la copie cloud.</div></div>";
      let noun = d.count > 1 ? "fichiers" : "fichier";
      banner += '<div class="del-count"><span class="del-big">' + d.count + "</span> " + noun + " \xB7 <b>" + esc(fmtSize(d.size) || "0 o") + "</b> \xE0 supprimer localement" + (d.truncated ? " (aper\xE7u partiel)" : "") + "</div>";
      document.getElementById("del-summary").innerHTML = banner;
      let fl = "";
      for (let i = 0; i < d.files.length; i++) {
        let f = d.files[i];
        f.action = "deleted";
        f.path = d.is_dir ? d.path === "." ? f.path : d.path + "/" + f.path : d.path;
        fl += renderFileRow(f, "", { hideTime: true, hideAction: true });
      }
      if (d.truncated) fl += `<div class="del-more">\u2026 et d'autres fichiers non list\xE9s</div>`;
      document.getElementById("del-files").innerHTML = fl;
      document.getElementById("del-drive").innerHTML = '<button class="btn btn-g del-drive-btn" onclick="runDriveCheck()">' + ICO_CLOUD + " Comparer avec le Drive (rclone check)</button>";
      document.getElementById("del-confirm").disabled = false;
    }).catch(function() {
      document.getElementById("del-summary").innerHTML = '<div class="del-banner danger">Serveur injoignable</div>';
    });
  }
  function runDriveCheck() {
    let el = document.getElementById("del-drive");
    el.innerHTML = '<div class="del-drive-load"><span class="fm-spin"></span> Comparaison avec le Drive en cours\u2026</div>';
    fetch("/api/drive_check?path=" + encodeURIComponent(_del.path)).then(function(r) {
      return r.json();
    }).then(function(d) {
      if (!d.ok) {
        el.innerHTML = '<div class="del-banner warn">' + ICO_WARN + "<div>Comparaison impossible : " + esc(d.error || "") + "</div></div>";
        return;
      }
      if (!d.exists) {
        el.innerHTML = '<div class="del-banner danger">' + ICO_WARN + "<div><b>Absent du Drive.</b> Aucune copie cloud d\xE9tect\xE9e : supprimer localement perdrait d\xE9finitivement ces donn\xE9es.</div></div>";
        return;
      }
      let c = d.counts;
      if (d.fully_backed) {
        el.innerHTML = '<div class="del-banner ok">' + ICO_CHECK + "<div><b>Int\xE9gralement pr\xE9sent sur le Drive.</b> " + c.identical + " fichier(s) identiques" + (c.drive_only ? " \xB7 " + c.drive_only + " en plus c\xF4t\xE9 Drive" : "") + ". Suppression locale sans perte.</div></div>";
        return;
      }
      let lines = "";
      if (c.local_only)
        lines += '<div class="dc-line danger">' + c.local_only + " fichier(s) uniquement en local \u2014 seraient perdus</div>";
      if (c.differ)
        lines += '<div class="dc-line warn">' + c.differ + " fichier(s) diff\xE9rents de la version Drive</div>";
      if (c.error) lines += '<div class="dc-line warn">' + c.error + " erreur(s) de lecture</div>";
      if (c.identical)
        lines += '<div class="dc-line ok">' + c.identical + " fichier(s) identiques</div>";
      let det = "";
      (d.result.local_only || []).slice(0, 50).forEach(function(n) {
        det += '<div class="dc-item danger">+ ' + esc(n) + "</div>";
      });
      (d.result.differ || []).slice(0, 50).forEach(function(n) {
        det += '<div class="dc-item warn">\u2260 ' + esc(n) + "</div>";
      });
      el.innerHTML = '<div class="del-banner warn">' + ICO_WARN + '<div><b>Diff\xE9rences d\xE9tect\xE9es.</b> Certains fichiers locaux ne sont pas (ou pas \xE0 jour) sur le Drive.</div></div><div class="dc-summary">' + lines + "</div>" + (det ? '<div class="dc-detail">' + det + "</div>" : "");
    }).catch(function() {
      el.innerHTML = '<div class="del-banner warn">Serveur injoignable</div>';
    });
  }
  function confirmDelete() {
    let cb = document.getElementById("del-confirm");
    cb.disabled = true;
    cb.textContent = "Suppression\u2026";
    fetch("/api/delete", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: _del.path })
    }).then(function(r) {
      return r.json();
    }).then(function(d) {
      cb.textContent = "Supprimer localement";
      if (d.ok) {
        toast("Supprim\xE9 localement \u2014 " + (fmtSize(d.freed) || "0 o") + " lib\xE9r\xE9s", "ok");
        closeDeleteModal();
        if (document.getElementById("tree-modal").classList.contains("show")) loadTree(_fm.dir);
      } else {
        cb.disabled = false;
        toast("\xC9chec de la suppression : " + (d.error || ""), "err");
      }
    }).catch(function() {
      cb.disabled = false;
      cb.textContent = "Supprimer localement";
      toast("Serveur injoignable", "err");
    });
  }
  function closeDeleteModal() {
    document.getElementById("delete-modal").classList.remove("show");
  }

  // src/js/exclude.js
  var _exc = { path: "", isDir: false };
  function ignorePath(path, isDir) {
    openExcludeModal(path, isDir);
  }
  function openExcludeModal(path, isDir) {
    _exc = { path, isDir: !!isDir };
    document.getElementById("exclude-modal").classList.add("show");
    document.getElementById("exc-path").textContent = "/" + path;
    let noneRadio = document.querySelector('input[name="exc-action"][value="none"]');
    if (noneRadio) noneRadio.checked = true;
    excOnChoice();
    document.getElementById("exc-summary").innerHTML = '<div class="del-loading">Analyse du contenu\u2026</div>';
    document.getElementById("exc-files").innerHTML = "";
    fetch("/api/delete_preview?path=" + encodeURIComponent(path)).then(function(r) {
      return r.json();
    }).then(function(d) {
      if (!d.ok) {
        document.getElementById("exc-summary").innerHTML = '<div class="del-count">La r\xE8gle <span class="mono">' + esc(excRule()) + "</span> sera ajout\xE9e.</div>";
        return;
      }
      let noun = d.count > 1 ? "fichiers" : "fichier";
      document.getElementById("exc-summary").innerHTML = '<div class="del-count">Concerne <span class="del-big2">' + d.count + "</span> " + noun + " \xB7 <b>" + esc(fmtSize(d.size) || "0 o") + "</b> en local" + (d.truncated ? " (aper\xE7u partiel)" : "") + '</div><div class="exc-rule">R\xE8gle ajout\xE9e : <span class="mono">' + esc(excRule()) + "</span></div>";
      let fl = "";
      d.files.forEach(function(f) {
        f.action = "excluded";
        f.path = d.is_dir ? d.path === "." ? f.path : d.path + "/" + f.path : d.path;
        fl += renderFileRow(f, "", { hideTime: true, hideAction: true });
      });
      if (d.truncated) fl += `<div class="del-more">\u2026 et d'autres fichiers non list\xE9s</div>`;
      document.getElementById("exc-files").innerHTML = fl;
    }).catch(function() {
      document.getElementById("exc-summary").innerHTML = `<div class="del-count">Serveur injoignable pour l'aper\xE7u.</div>`;
    });
  }
  function excRule() {
    return _exc.isDir ? "- " + _exc.path + "/**" : "- " + _exc.path;
  }
  function excChoice() {
    let el = document.querySelector('input[name="exc-action"]:checked');
    return el ? el.value : "none";
  }
  function excOnChoice() {
    let v = excChoice();
    let btn = document.getElementById("exc-confirm");
    let labels = {
      none: "Exclure",
      local: "Exclure et supprimer en local",
      drive: "Exclure et supprimer du Drive",
      both: "Exclure et supprimer des deux"
    };
    btn.textContent = labels[v] || "Exclure";
    btn.className = "btn " + (v === "none" ? "btn-g" : "btn-danger");
    btn.disabled = false;
  }
  async function confirmExclude() {
    let choice = excChoice();
    let btn = document.getElementById("exc-confirm");
    btn.disabled = true;
    let origLabel = btn.textContent;
    btn.textContent = "En cours\u2026";
    try {
      let r = await fetch("/api/filters_add?rule=" + encodeURIComponent(excRule()), { method: "POST" });
      let d = await r.json();
      if (!d.ok) {
        toast("Impossible d'exclure : " + (d.error || ""), "err");
        btn.disabled = false;
        btn.textContent = origLabel;
        return;
      }
      let msgs = ["Exclu de la synchronisation"];
      if (choice === "local" || choice === "both") {
        let rl = await fetch("/api/delete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path: _exc.path })
        });
        let dl = await rl.json();
        if (dl.ok) msgs.push("supprim\xE9 en local (" + (fmtSize(dl.freed) || "0 o") + ")");
        else toast("Suppression locale \xE9chou\xE9e : " + (dl.error || ""), "err");
      }
      if (choice === "drive" || choice === "both") {
        let rd = await fetch("/api/drive_delete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ path: _exc.path, is_dir: _exc.isDir })
        });
        let dd = await rd.json();
        if (dd.ok) msgs.push("supprim\xE9 du Drive");
        else toast("Suppression Drive \xE9chou\xE9e : " + (dd.error || ""), "err");
      }
      toast(msgs.join(" \xB7 "), "ok");
      closeExcludeModal();
      if (document.getElementById("tree-modal").classList.contains("show")) loadTree(_fm.dir);
    } catch (e) {
      toast("Serveur injoignable", "err");
      btn.disabled = false;
      btn.textContent = origLabel;
    }
  }
  function closeExcludeModal() {
    document.getElementById("exclude-modal").classList.remove("show");
  }
  async function reincludePath(path, isDir) {
    let exact = isDir ? path + "/**" : path;
    try {
      let r = await fetch("/api/match_rules?path=" + encodeURIComponent(path));
      let d = await r.json();
      if (!d.ok) {
        toast("Erreur : " + (d.error || ""), "err");
        return;
      }
      let rules = d.rules || [];
      if (rules.length === 0) {
        toast("Cet \xE9l\xE9ment n'est plus exclu", "ok");
        loadTree(_fm.dir);
        return;
      }
      let others = rules.filter(function(x) {
        return x !== exact;
      });
      if (others.length === 0) {
        let rr = await fetch("/api/filters_remove?rule=" + encodeURIComponent("- " + exact), { method: "POST" });
        let dr = await rr.json();
        if (dr.ok && dr.removed) {
          toast("R\xE9-inclus dans la synchronisation", "ok");
          loadTree(_fm.dir);
        } else {
          toast("Impossible de retirer la r\xE8gle", "err");
        }
        return;
      }
      openReincModal(path, rules);
    } catch (e) {
      toast("Serveur injoignable", "err");
    }
  }
  function openReincModal(path, rules) {
    document.getElementById("reinc-modal").classList.add("show");
    document.getElementById("reinc-summary").innerHTML = '<div class="del-banner warn">' + ICO_WARN + "<div><b>" + esc(path) + "</b> n'est pas exclu par une r\xE8gle qui lui est propre, mais par " + (rules.length > 1 ? "ces motifs g\xE9n\xE9raux" : "ce motif g\xE9n\xE9ral") + " :</div></div>";
    let html = "";
    rules.forEach(function(rp) {
      html += '<div class="del-file"><span class="df-name">- ' + esc(rp) + "</span></div>";
    });
    document.getElementById("reinc-rules").innerHTML = html;
  }
  function closeReincModal() {
    document.getElementById("reinc-modal").classList.remove("show");
  }

  // src/js/filters.js
  async function openFiltersModal() {
    document.getElementById("filters-modal").classList.add("show");
    await loadFilters();
  }
  function closeFiltersModal() {
    document.getElementById("filters-modal").classList.remove("show");
  }
  var _originalFiltersText = "";
  function checkFiltersModified() {
    let tf = document.getElementById("filters-text");
    let btn = document.getElementById("save-filters-btn");
    btn.disabled = tf.value === _originalFiltersText;
  }
  async function loadFilters() {
    let tf = document.getElementById("filters-text");
    tf.value = "Chargement\u2026";
    try {
      let r = await fetch("/api/filters");
      let d = await r.json();
      tf.value = d.content || (d.error ? "Erreur : " + d.error : "");
      _originalFiltersText = tf.value;
      checkFiltersModified();
      tf.scrollTop = tf.scrollHeight;
    } catch (e) {
      tf.value = "Serveur injoignable";
    }
  }
  function addFilter() {
    let input = document.getElementById("new-filter-input");
    let rule = input.value.trim();
    if (!rule) return;
    if (!rule.startsWith("- ") && !rule.startsWith("+ ") && !rule.startsWith("#")) {
      rule = "- " + rule;
    }
    if (rule.startsWith("#") || rule.startsWith("+ ")) {
      commitFilter(rule);
      return;
    }
    openImpactModal(rule);
  }
  function commitFilter(rule) {
    let input = document.getElementById("new-filter-input");
    let tf = document.getElementById("filters-text");
    tf.value += (tf.value.endsWith("\n") || !tf.value ? "" : "\n") + rule + "\n";
    input.value = "";
    tf.scrollTop = tf.scrollHeight;
    return saveFilters();
  }
  var _pendingFilter = null;
  function openImpactModal(rule) {
    _pendingFilter = rule;
    document.getElementById("impact-modal").classList.add("show");
    document.getElementById("impact-rule").textContent = rule;
    let noneRadio = document.querySelector('input[name="imp-action"][value="none"]');
    if (noneRadio) noneRadio.checked = true;
    impOnChoice();
    document.getElementById("impact-summary").innerHTML = `<div class="del-loading">Analyse de l'impact\u2026</div>`;
    document.getElementById("impact-files").innerHTML = "";
    document.getElementById("impact-confirm").disabled = true;
    fetch("/api/rule_impact?rule=" + encodeURIComponent(rule)).then(function(r) {
      return r.json();
    }).then(function(d) {
      if (!d.ok) {
        document.getElementById("impact-summary").innerHTML = '<div class="del-count">Impact non calculable : ' + esc(d.error || "") + "</div>";
        document.getElementById("impact-confirm").disabled = false;
        return;
      }
      let summary;
      if (d.count === 0) {
        summary = '<div class="del-banner ok">' + ICO_CHECK + "<div>Aucun \xE9l\xE9ment pr\xE9sent ne correspond pour l'instant. La r\xE8gle s'appliquera aux futurs fichiers.</div></div>";
      } else {
        let noun = d.count > 1 ? "\xE9l\xE9ments" : "\xE9l\xE9ment";
        summary = '<div class="del-count">Cette r\xE8gle exclut <span class="del-big2">' + d.count + "</span> " + noun + " \xB7 <b>" + esc(fmtSize(d.size) || "0 o") + "</b>" + (d.truncated ? " (aper\xE7u partiel)" : "") + "</div>";
      }
      document.getElementById("impact-summary").innerHTML = summary;
      let fl = "";
      d.items.forEach(function(it) {
        let meta = it.is_dir ? (it.count || 0) + (it.count > 1 ? " fichiers" : " fichier") : fmtSize(it.size) || "";
        it.action = "excluded";
        fl += renderFileRow(it, "", { hideTime: true, customSize: meta, hideAction: true });
      });
      if (d.truncated) fl += `<div class="del-more">\u2026 et d'autres \xE9l\xE9ments non list\xE9s</div>`;
      document.getElementById("impact-files").innerHTML = fl;
      document.getElementById("impact-confirm").disabled = false;
    }).catch(function() {
      document.getElementById("impact-summary").innerHTML = '<div class="del-count">Serveur injoignable</div>';
      document.getElementById("impact-confirm").disabled = false;
    });
  }
  function closeImpactModal() {
    document.getElementById("impact-modal").classList.remove("show");
    _pendingFilter = null;
  }
  function impChoice() {
    let el = document.querySelector('input[name="imp-action"]:checked');
    return el ? el.value : "none";
  }
  function impOnChoice() {
    let v = impChoice();
    let btn = document.getElementById("impact-confirm");
    let labels = {
      none: "Ajouter cette exclusion",
      local: "Exclure et supprimer en local",
      drive: "Exclure et supprimer du Drive",
      both: "Exclure et supprimer des deux"
    };
    btn.textContent = labels[v] || "Ajouter cette exclusion";
    btn.className = "btn " + (v === "none" ? "btn-g" : "btn-danger");
  }
  async function confirmAddFilter() {
    let rule = _pendingFilter;
    if (!rule) {
      closeImpactModal();
      return;
    }
    let choice = impChoice();
    let btn = document.getElementById("impact-confirm");
    btn.disabled = true;
    btn.textContent = "En cours\u2026";
    try {
      await commitFilter(rule);
      if (choice !== "none") {
        let r = await fetch("/api/rule_delete", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ rule, mode: choice })
        });
        let d = await r.json();
        if (d.ok) {
          let parts = [
            d.count + " \xE9l\xE9ment" + (d.count > 1 ? "s" : "") + " trait\xE9" + (d.count > 1 ? "s" : "")
          ];
          if (choice !== "drive") parts.push((fmtSize(d.freed) || "0 o") + " lib\xE9r\xE9s");
          if (d.truncated) parts.push("liste tronqu\xE9e");
          let hasErr = d.errors && d.errors.length;
          if (hasErr) parts.push(d.errors.length + " erreur(s)");
          toast("Exclusion appliqu\xE9e \xB7 " + parts.join(" \xB7 "), hasErr ? "warn" : "ok");
        } else {
          toast("Suppression \xE9chou\xE9e : " + (d.error || ""), "err");
        }
      }
      closeImpactModal();
      if (document.getElementById("tree-modal").classList.contains("show")) loadTree(_fm.dir);
    } catch (e) {
      toast("Serveur injoignable", "err");
      btn.disabled = false;
      impOnChoice();
    }
  }
  async function saveFilters() {
    let tf = document.getElementById("filters-text");
    let btn = document.getElementById("save-filters-btn");
    btn.disabled = true;
    btn.textContent = "Enregistrement\u2026";
    try {
      let r = await fetch("/api/filters_save", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: tf.value })
      });
      let d = await r.json();
      if (d.ok) {
        toast("Exclusions enregistr\xE9es", "ok");
        _originalFiltersText = tf.value;
        checkFiltersModified();
      } else {
        toast("Enregistrement impossible : " + d.error, "err");
      }
    } catch (e) {
      toast("Serveur injoignable \u2014 exclusions non enregistr\xE9es", "err");
    } finally {
      btn.textContent = "Enregistrer";
      checkFiltersModified();
    }
  }

  // src/js/modals.js
  function openSettingsModal() {
    document.getElementById("settings-modal").classList.add("show");
    fetch("/api/settings").then((r) => r.json()).then((d) => {
      if (d.remote != null) document.getElementById("set-remote").value = d.remote;
      if (d.local_dir != null) document.getElementById("set-local-dir").value = d.local_dir;
      if (d.timer_interval != null) document.getElementById("set-timer").value = d.timer_interval;
      if (d.bwlimit != null) document.getElementById("set-bwlimit").value = d.bwlimit;
    });
  }
  function closeSettingsModal() {
    document.getElementById("settings-modal").classList.remove("show");
  }
  async function saveSettings() {
    let btn = document.getElementById("btn-save-settings");
    let data = {
      remote: document.getElementById("set-remote").value.trim(),
      local_dir: document.getElementById("set-local-dir").value.trim(),
      timer_interval: document.getElementById("set-timer").value,
      bwlimit: document.getElementById("set-bwlimit").value
    };
    if (!data.remote || !data.local_dir) {
      toast("La cible et le dossier local sont requis.", "err");
      return;
    }
    btn.disabled = true;
    btn.innerHTML = "Enregistrement...";
    try {
      let r = await fetch("/api/settings_save", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data)
      });
      let d = await r.json();
      if (d.ok) {
        toast("Param\xE8tres appliqu\xE9s. Red\xE9marrage du Dashboard...", "ok");
        setTimeout(() => {
          window.location.reload();
        }, 2e3);
      } else {
        toast("Impossible d'appliquer : " + d.error, "err");
        btn.disabled = false;
        btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path><polyline points="17 21 17 13 7 13 7 21"></polyline><polyline points="7 3 7 8 15 8"></polyline></svg> Enregistrer & Red\xE9marrer';
      }
    } catch (e) {
      toast("Param\xE8tres appliqu\xE9s. Red\xE9marrage du Dashboard...", "ok");
      setTimeout(() => {
        window.location.reload();
      }, 2e3);
    }
  }
  function openDryRunModal() {
    document.getElementById("dryrun-modal").classList.add("show");
  }
  function closeDryRunModal() {
    document.getElementById("dryrun-modal").classList.remove("show");
  }
  async function startDryRun() {
    let btn = document.getElementById("start-dryrun-btn");
    let out = document.getElementById("dryrun-output");
    btn.disabled = true;
    btn.textContent = "Analyse en cours\u2026";
    out.classList.remove("is-empty");
    out.textContent = "Analyse des diff\xE9rences entre le dossier local et Google Drive\u2026\nCela peut prendre une \xE0 deux minutes.";
    try {
      let r = await fetch("/api/dryrun");
      let d = await r.json();
      if (d.ok) {
        out.innerHTML = colorizeLog(
          d.log || "Aucun changement \xE0 appliquer : tout est d\xE9j\xE0 synchronis\xE9."
        );
      } else {
        out.innerHTML = colorizeLog("La simulation a \xE9chou\xE9 :\n" + d.error);
      }
    } catch (e) {
      out.textContent = "Serveur injoignable : " + e.message;
    } finally {
      btn.disabled = false;
      btn.textContent = "Relancer la simulation";
    }
  }

  // src/js/main.js
  Object.assign(window, {
    // refresh + sync
    doSync,
    cancelSync,
    refresh,
    // theme
    toggleTheme,
    // logs
    setLogFilter,
    logsBot,
    // recent
    filterRecent,
    // sparkline tooltips
    showTooltip,
    hideTooltip,
    // history
    toggleRunDetails,
    copyErrorLogs,
    // file browser
    openTreeModal,
    closeTreeModal,
    treeUp,
    loadTree,
    openCurrentDir,
    fmFilter,
    fmClearSearch,
    fmToggleRecursive,
    fmSort,
    openFile,
    _fm,
    // delete modal
    openDeleteModal,
    closeDeleteModal,
    confirmDelete,
    runDriveCheck,
    // exclude modal
    ignorePath,
    openExcludeModal,
    closeExcludeModal,
    confirmExclude,
    excOnChoice,
    reincludePath,
    closeReincModal,
    // filters modal
    openFiltersModal,
    closeFiltersModal,
    addFilter,
    saveFilters,
    checkFiltersModified,
    openImpactModal,
    closeImpactModal,
    impOnChoice,
    confirmAddFilter,
    // config & dry run modals
    openSettingsModal,
    closeSettingsModal,
    saveSettings,
    openDryRunModal,
    closeDryRunModal,
    startDryRun
  });
  document.addEventListener("keydown", function(e) {
    if (e.key === "Escape") {
      document.querySelectorAll(".modal-overlay.show").forEach(function(m) {
        m.classList.remove("show");
      });
    }
  });
  document.addEventListener("click", function(e) {
    let el = e.target.closest(".file-link[data-openfile]");
    if (el) openFile(el.dataset.fpath, el.dataset.deleted === "1", e);
  });
  function updateCtrlState(e) {
    let ctrl = e.ctrlKey || e.metaKey;
    document.body.classList.toggle("folder-mode", ctrl);
    document.body.classList.toggle("folder-mode-all", ctrl && e.shiftKey);
  }
  window.addEventListener("keydown", updateCtrlState);
  window.addEventListener("keyup", updateCtrlState);
  window.addEventListener("blur", function() {
    document.body.classList.remove("folder-mode", "folder-mode-all");
  });
  applyThemeIcon();
  initDragAndDrop();
  initFocusTrap();
  refresh();
  initLiveStream();
  S.interval = setInterval(refresh, 1e4);
  setInterval(tickPulse, 1e3);
})();
