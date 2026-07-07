/* ═══════════════════════════════════════════════════
   MODULES DÉPLAÇABLES (drag & drop + redimensionnement)
   ═══════════════════════════════════════════════════ */
var dragSrcEl = null;

function handleDragStart(e) {
  dragSrcEl = this;
  e.dataTransfer.effectAllowed = 'move';
  e.dataTransfer.setData('text/html', this.innerHTML);
  this.classList.add('dragging');
}

function handleDragOver(e) {
  if (e.preventDefault) { e.preventDefault(); }
  e.dataTransfer.dropEffect = 'move';
  return false;
}

function handleDragEnter(e) { this.classList.add('over'); }
function handleDragLeave(e) { this.classList.remove('over'); }

function handleDrop(e) {
  if (e.stopPropagation) { e.stopPropagation(); }
  if (dragSrcEl !== this) {
    var srcOrder = window.getComputedStyle(dragSrcEl).order;
    var destOrder = window.getComputedStyle(this).order;

    if (srcOrder === destOrder) {
      var panels = document.querySelectorAll('.drag-panel');
      panels.forEach(function (p, i) { p.style.order = p.style.order || i; });
      srcOrder = dragSrcEl.style.order;
      destOrder = this.style.order;
    }

    dragSrcEl.style.order = destOrder;
    this.style.order = srcOrder;

    var orderData = {};
    document.querySelectorAll('.drag-panel').forEach(function (p) {
      p.style.height = '';
      orderData[p.id] = p.style.order;
    });
    localStorage.setItem('dash_panel_order', JSON.stringify(orderData));
    updateFullWidthPanel();
  }
  return false;
}

function handleDragEnd(e) {
  this.classList.remove('dragging');
  document.querySelectorAll('.drag-panel').forEach(function (p) { p.classList.remove('over'); });
}

function initDragAndDrop() {
  var panels = document.querySelectorAll('.drag-panel');
  var savedOrder = JSON.parse(localStorage.getItem('dash_panel_order') || '{}');

  panels.forEach(function (panel, idx) {
    panel.style.order = savedOrder[panel.id] || idx;

    var header = panel.querySelector('.ph');
    if (header) {
      header.addEventListener('mouseenter', function () { panel.setAttribute('draggable', 'true'); });
      header.addEventListener('mouseleave', function () { panel.removeAttribute('draggable'); });
    }

    panel.addEventListener('dragstart', handleDragStart, false);
    panel.addEventListener('dragenter', handleDragEnter, false);
    panel.addEventListener('dragover', handleDragOver, false);
    panel.addEventListener('dragleave', handleDragLeave, false);
    panel.addEventListener('drop', handleDrop, false);
    panel.addEventListener('dragend', handleDragEnd, false);
  });

  updateFullWidthPanel();
  initResizer();
}

function updateFullWidthPanel() {
  var panels = Array.from(document.querySelectorAll('.drag-panel'));
  panels.sort(function (a, b) { return parseInt(a.style.order || 0) - parseInt(b.style.order || 0); });
  panels.forEach(function (p, idx) {
    p.classList.toggle('full-width', idx === 2);
  });
}

function initResizer() {
  var container = document.getElementById('modules-container');
  var isResizingH = false;
  var isResizingV = false;
  var startX, startY, startCol1, startCol2, startRow1, startRow2;

  var savedCol = localStorage.getItem('dash_col_ratio');
  if (savedCol) {
    var parts = savedCol.split(':');
    container.style.setProperty('--col1', parts[0] + 'fr');
    container.style.setProperty('--col2', parts[1] + 'fr');
  }
  var savedRow = localStorage.getItem('dash_row_sizes');
  if (savedRow) {
    var rParts = savedRow.split(':');
    container.style.setProperty('--row1', rParts[0] + 'px');
    container.style.setProperty('--row2', rParts[1] + 'px');
  }

  var hoverRaf = 0;
  var lastMx = 0, lastMy = 0;

  function updateHoverCursor(clientX, clientY) {
    var panels = Array.from(document.querySelectorAll('.drag-panel'));
    panels.sort(function (a, b) { return parseInt(a.style.order || 0) - parseInt(b.style.order || 0); });
    if (panels.length < 3) return;
    var p1 = panels[0].getBoundingClientRect();
    var p2 = panels[1].getBoundingClientRect();
    var p3 = panels[2].getBoundingClientRect();

    var isH = (clientX > p1.right - 5 && clientX < p2.left + 5 && clientY > p1.top && clientY < p1.bottom);
    var isV = (clientY > p1.bottom - 5 && clientY < p3.top + 5 && clientX > p3.left && clientX < p3.right);

    var cursor = isH && isV ? 'move' : isH ? 'col-resize' : isV ? 'row-resize' : '';
    if (container.style.cursor !== cursor) container.style.cursor = cursor;
  }

  container.addEventListener('mousemove', function (e) {
    if (isResizingH) {
      var rect = container.getBoundingClientRect();
      var dx = e.clientX - startX;
      var c1 = startCol1 + (dx / rect.width) * (startCol1 + startCol2);
      var c2 = startCol2 - (dx / rect.width) * (startCol1 + startCol2);
      if (c1 > 0.1 && c2 > 0.1) {
        container.style.setProperty('--col1', c1 + 'fr');
        container.style.setProperty('--col2', c2 + 'fr');
        localStorage.setItem('dash_col_ratio', c1 + ':' + c2);
      }
      return;
    }
    if (isResizingV) {
      var dy = e.clientY - startY;
      var r1 = startRow1 + dy;
      var r2 = startRow2 - dy;
      if (r1 > 100 && r2 > 100) {
        container.style.setProperty('--row1', r1 + 'px');
        container.style.setProperty('--row2', r2 + 'px');
        localStorage.setItem('dash_row_sizes', r1 + ':' + r2);
      }
      return;
    }

    lastMx = e.clientX; lastMy = e.clientY;
    if (hoverRaf) return;
    hoverRaf = requestAnimationFrame(function () {
      hoverRaf = 0;
      updateHoverCursor(lastMx, lastMy);
    });
  });

  container.addEventListener('mousedown', function (e) {
    if (container.style.cursor === 'col-resize' || container.style.cursor === 'move') {
      isResizingH = true;
      startX = e.clientX;
      startCol1 = parseFloat(getComputedStyle(container).getPropertyValue('--col1')) || 1;
      startCol2 = parseFloat(getComputedStyle(container).getPropertyValue('--col2')) || 1;
      e.preventDefault();
    }
    if (container.style.cursor === 'row-resize' || container.style.cursor === 'move') {
      isResizingV = true;
      startY = e.clientY;
      startRow1 = parseFloat(getComputedStyle(container).getPropertyValue('--row1')) || 300;
      startRow2 = parseFloat(getComputedStyle(container).getPropertyValue('--row2')) || 260;
      e.preventDefault();
    }
    if (isResizingH || isResizingV) document.body.style.cursor = container.style.cursor;
  });

  window.addEventListener('mouseup', function () {
    isResizingH = false;
    isResizingV = false;
    document.body.style.cursor = '';
  });
}
