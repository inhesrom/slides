/// Generate the complete editor page HTML with inline CSS and JS.
pub fn editor_html() -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>slides — Editor</title>
  <style>{css}</style>
</head>
<body>
  <div class="editor-app">
    <header class="topbar">
      <div class="topbar-left">
        <span class="logo">slides</span>
        <span class="mode-badge">EDITOR</span>
      </div>
      <div class="topbar-center" id="deck-settings">
        <label>Title <input type="text" id="cfg-title" placeholder="Untitled" oninput="scheduleSave()"></label>
        <label>Theme <select id="cfg-theme" onchange="scheduleSave()"><option value="minimal">Minimal</option><option value="dark">Dark</option></select></label>
        <label>Aspect <select id="cfg-aspect" onchange="scheduleSave()"><option value="16:9">16:9</option><option value="4:3">4:3</option></select></label>
        <label>Transition <select id="cfg-transition" onchange="scheduleSave()"><option value="slide">Slide</option><option value="fade">Fade</option><option value="none">None</option></select></label>
        <label>Color <select id="cfg-color" onchange="scheduleSave()"><option value="light">Light</option><option value="dark">Dark</option></select></label>
        <label>Title Size <input type="number" id="cfg-title-size" class="size-input" min="20" max="200" step="1" oninput="scheduleSave()"> px</label>
        <label>Body Size <input type="number" id="cfg-body-size" class="size-input" min="10" max="120" step="1" oninput="scheduleSave()"> px</label>
      </div>
      <div class="topbar-right">
        <span class="status" id="save-status">Connected</span>
      </div>
    </header>
    <div class="main-panels">
      <aside class="slide-list" id="slide-list">
        <div class="slide-list-header">
          <span>Slides</span>
          <button class="btn-icon" onclick="addSlide()" title="Add slide">+</button>
        </div>
        <div class="slide-list-items" id="slide-list-items"></div>
      </aside>
      <section class="edit-panel" id="edit-panel">
        <div class="edit-header">
          <span id="edit-label">Slide 1</span>
          <div class="edit-controls">
            <button class="btn-sm" onclick="moveSlideUp()" title="Move up">&#9650;</button>
            <button class="btn-sm" onclick="moveSlideDown()" title="Move down">&#9660;</button>
            <button class="btn-sm btn-danger" onclick="deleteSlide()" title="Delete slide">&#10005;</button>
          </div>
        </div>
        <div class="field-row">
          <label>Layout</label>
          <select id="layout-select" onchange="onLayoutChange()">
            <option value="none">None</option>
            <option value="split">Split (columns)</option>
            <option value="grid">Grid</option>
            <option value="stack">Stack (vertical)</option>
          </select>
          <input type="text" id="layout-params" placeholder="e.g. 60/40 or 2x2" class="layout-params-input" oninput="onLayoutParamsChange()">
        </div>
        <div class="field-row">
          <label>Slide Transition</label>
          <select id="slide-transition" onchange="scheduleSave()">
            <option value="">Default</option>
            <option value="slide">Slide</option>
            <option value="fade">Fade</option>
            <option value="none">None</option>
          </select>
          <label class="checkbox-label"><input type="checkbox" id="slide-centered" onchange="onCenteredChange()"> Centered</label>
        </div>
        <div class="toolbar" id="toolbar">
          <button onclick="toolBold()" title="Bold (Ctrl+B)"><b>B</b></button>
          <button onclick="toolItalic()" title="Italic (Ctrl+I)"><i>I</i></button>
          <button onclick="toolStrike()" title="Strikethrough"><s>S</s></button>
          <span class="toolbar-sep"></span>
          <button onclick="toolHeading(1)" title="Heading 1">H1</button>
          <button onclick="toolHeading(2)" title="Heading 2">H2</button>
          <button onclick="toolHeading(3)" title="Heading 3">H3</button>
          <span class="toolbar-sep"></span>
          <button onclick="toolBulletList()" title="Bullet list">&bull;</button>
          <button onclick="toolNumberedList()" title="Numbered list">1.</button>
          <button onclick="toolFragmentList()" title="Fragment (reveal)">+</button>
          <span class="toolbar-sep"></span>
          <button onclick="toolBlockquote()" title="Blockquote">&ldquo;</button>
          <button onclick="toolCodeBlock()" title="Code block">&lt;/&gt;</button>
          <button onclick="toolLink()" title="Insert link">&#128279;</button>
          <button onclick="toolImage()" title="Insert image">&#128247;</button>
          <button onclick="toolTable()" title="Insert table">&#9638;</button>
        </div>
        <div class="overflow-warning hidden" id="overflow-warning">Slide content exceeds available space</div>
        <div id="content-area">
          <textarea id="content" class="editor-textarea" placeholder="Slide content (markdown)..." oninput="scheduleSave()"></textarea>
        </div>
        <div id="regions-area" class="regions-area hidden"></div>
        <div class="notes-section">
          <label>Speaker Notes</label>
          <textarea id="notes" class="editor-textarea notes-textarea" placeholder="Speaker notes..." oninput="scheduleSave()"></textarea>
        </div>
      </section>
      <aside class="preview-panel" id="preview-panel">
        <div class="resize-handle" id="resize-handle"></div>
        <div class="preview-header">
          <span>Live Preview</span>
          <label class="checkbox-label" title="Force all fragments visible in preview"><input type="checkbox" id="preview-reveal-all" checked onchange="onPreviewRevealChange()"> Reveal all</label>
          <button class="btn-sm" onclick="togglePreview()" id="preview-toggle" title="Hide preview">&#9654;</button>
        </div>
        <div class="preview-container" id="preview-container">
          <iframe id="preview-frame" src="/"></iframe>
        </div>
      </aside>
      <button class="preview-restore" id="preview-restore" onclick="togglePreview()" title="Show preview">&#9664;</button>
    </div>
  </div>
  <input type="file" id="file-input" style="display:none" accept="image/*" onchange="handleFileSelect(event)">
  <script>{js}</script>
</body>
</html>"##,
        css = EDITOR_CSS,
        js = EDITOR_JS,
    )
}

const EDITOR_CSS: &str = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: system-ui, -apple-system, sans-serif; background: #0f172a; color: #e2e8f0; height: 100vh; overflow: hidden; }

.editor-app { display: flex; flex-direction: column; height: 100vh; }

/* Top bar */
.topbar { display: flex; align-items: center; justify-content: space-between; padding: 0.5rem 1rem; background: #1e293b; border-bottom: 1px solid #334155; gap: 1rem; flex-shrink: 0; }
.topbar-left { display: flex; align-items: center; gap: 0.75rem; }
.logo { font-weight: 700; font-size: 1.1rem; color: #f1f5f9; }
.mode-badge { font-size: 0.65rem; font-weight: 700; letter-spacing: 0.1em; background: #7dd3fc; color: #0f172a; padding: 0.15rem 0.5rem; border-radius: 3px; }
.topbar-center { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
.topbar-center label { display: flex; align-items: center; gap: 0.35rem; font-size: 0.75rem; color: #94a3b8; }
.topbar-center input, .topbar-center select { background: #0f172a; border: 1px solid #334155; color: #e2e8f0; padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.8rem; }
.topbar-center input[type="text"] { width: 140px; }
.topbar-center input.size-input { width: 4em; }
.topbar-right { display: flex; align-items: center; }
.status { font-size: 0.75rem; color: #22c55e; }
.status.error { color: #ef4444; }
.status.saving { color: #eab308; }

/* Main layout */
.main-panels { display: flex; flex: 1; overflow: hidden; position: relative; }

/* Slide list sidebar */
.slide-list { width: 180px; min-width: 180px; background: #1e293b; border-right: 1px solid #334155; display: flex; flex-direction: column; }
.slide-list-header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; font-size: 0.85rem; font-weight: 600; color: #94a3b8; border-bottom: 1px solid #334155; }
.slide-list-items { flex: 1; overflow-y: auto; padding: 0.5rem; }
.slide-card { padding: 0.5rem 0.6rem; margin-bottom: 0.35rem; border-radius: 6px; cursor: pointer; font-size: 0.8rem; color: #cbd5e1; border: 1px solid transparent; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.slide-card:hover { background: #334155; }
.slide-card.active { background: #334155; border-color: #7dd3fc; color: #f1f5f9; }
.slide-card .slide-num { color: #64748b; font-size: 0.7rem; margin-right: 0.4rem; }

/* Edit panel */
.edit-panel { flex: 1; display: flex; flex-direction: column; overflow: hidden; padding: 0.75rem; gap: 0.5rem; }
.edit-header { display: flex; justify-content: space-between; align-items: center; }
.edit-header span { font-size: 0.9rem; font-weight: 600; color: #94a3b8; }
.edit-controls { display: flex; gap: 0.25rem; }
.btn-sm { background: #334155; border: none; color: #e2e8f0; padding: 0.2rem 0.5rem; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }
.btn-sm:hover { background: #475569; }
.btn-danger:hover { background: #dc2626; }
.btn-icon { background: #334155; border: none; color: #e2e8f0; width: 26px; height: 26px; border-radius: 4px; cursor: pointer; font-size: 1rem; display: flex; align-items: center; justify-content: center; }
.btn-icon:hover { background: #475569; }

/* Fields */
.field-row { display: flex; align-items: center; gap: 0.5rem; }
.field-row label { font-size: 0.75rem; color: #94a3b8; min-width: 80px; }
.field-row select, .field-row input { background: #0f172a; border: 1px solid #334155; color: #e2e8f0; padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.8rem; }
.layout-params-input { width: 100px; }
.checkbox-label { display: flex; align-items: center; gap: 0.3rem; cursor: pointer; font-size: 0.8rem; color: #cbd5e1; }

/* Toolbar */
.toolbar { display: flex; gap: 2px; padding: 0.35rem; background: #1e293b; border-radius: 6px; flex-wrap: wrap; }
.toolbar button { background: #334155; border: none; color: #e2e8f0; padding: 0.3rem 0.55rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; min-width: 28px; }
.toolbar button:hover { background: #475569; }
.toolbar-sep { width: 1px; background: #475569; margin: 0 4px; }

/* Textareas */
.editor-textarea { width: 100%; background: #0f172a; border: 1px solid #334155; color: #e2e8f0; padding: 0.75rem; border-radius: 6px; font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 0.85rem; line-height: 1.6; resize: none; }
.editor-textarea:focus { outline: none; border-color: #7dd3fc; }
#content { flex: 1; min-height: 120px; }
#content-area { display: flex; flex-direction: column; flex: 1; min-height: 0; }
.notes-textarea { height: 80px; flex-shrink: 0; }
.notes-section { flex-shrink: 0; }
.notes-section label { font-size: 0.75rem; color: #94a3b8; display: block; margin-bottom: 0.25rem; }

/* Regions (for layouts) */
.regions-area { display: flex; flex-direction: column; gap: 0.5rem; flex: 1; min-height: 0; overflow-y: auto; }
.regions-area.hidden { display: none; }
.region-box { display: flex; flex-direction: column; flex: 1; min-height: 80px; }
.region-box label { font-size: 0.7rem; color: #64748b; margin-bottom: 0.2rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
.region-box textarea { flex: 1; width: 100%; background: #0f172a; border: 1px solid #334155; color: #e2e8f0; padding: 0.5rem; border-radius: 4px; font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 0.8rem; line-height: 1.5; resize: none; }
.region-box textarea:focus { outline: none; border-color: #7dd3fc; }

/* Preview */
.preview-panel { width: 38%; min-width: 200px; display: flex; flex-direction: column; border-left: 1px solid #334155; position: relative; transition: width 0.2s, min-width 0.2s; }
.preview-panel.collapsed { width: 0; min-width: 0; overflow: hidden; border-left: none; }
.preview-restore { display: none; position: absolute; right: 0; top: 50%; transform: translateY(-50%); background: #334155; border: 1px solid #475569; color: #e2e8f0; width: 24px; height: 48px; border-radius: 4px 0 0 4px; cursor: pointer; font-size: 0.75rem; z-index: 5; }
.preview-restore:hover { background: #475569; }
.preview-panel.collapsed ~ .preview-restore { display: block; }
.preview-header { padding: 0.5rem 0.75rem; font-size: 0.75rem; font-weight: 600; color: #94a3b8; background: #1e293b; border-bottom: 1px solid #334155; text-transform: uppercase; letter-spacing: 0.05em; display: flex; align-items: center; gap: 0.75rem; }
.preview-header > #preview-toggle { margin-left: auto; }
.preview-header .checkbox-label { text-transform: none; letter-spacing: normal; font-weight: 400; }
.preview-controls { display: flex; gap: 0.25rem; }

/* Scaled preview: render at full presentation size, scale down to fit */
.preview-container { flex: 1; position: relative; overflow: hidden; background: #1a1a2e; }
#preview-frame { position: absolute; top: 0; left: 0; width: 1920px; height: 1080px; border: none; transform-origin: top left; }

/* Resize handle — positioned at the left edge of preview panel */
.preview-panel { position: relative; }
.resize-handle { width: 5px; cursor: col-resize; background: transparent; position: absolute; left: -3px; top: 0; bottom: 0; z-index: 10; }
.resize-handle:hover, .resize-handle.dragging { background: #7dd3fc; }

/* Overflow warning */
.overflow-warning { background: #7f1d1d; color: #fca5a5; padding: 0.4rem 0.75rem; border-radius: 4px; font-size: 0.8rem; font-weight: 600; flex-shrink: 0; }
.overflow-warning.hidden { display: none; }

/* Drag over indicator */
.drag-over { border-color: #7dd3fc !important; background: #1e293b !important; }

/* Toast notifications */
.toast { position: fixed; bottom: 1rem; right: 1rem; background: #334155; color: #e2e8f0; padding: 0.6rem 1rem; border-radius: 6px; font-size: 0.85rem; z-index: 1000; opacity: 0; transition: opacity 0.3s; pointer-events: none; }
.toast.visible { opacity: 1; }
.toast.error { background: #7f1d1d; }
"##;

const EDITOR_JS: &str = r##"
(function() {
  'use strict';

  // --- State ---
  var deck = null;
  var selectedSlide = 0;
  var ws = null;
  var saveTimer = null;
  var SAVE_DEBOUNCE = 500;
  var pendingSave = false;
  var lastSaveTime = 0;
  var SAVE_SUPPRESS_WINDOW = 3000; // ms - ignore reloads after own save
  var lastPreviewRefresh = 0;
  var REFRESH_COOLDOWN = 800; // ms - ignore duplicate reloads

  // --- DOM refs ---
  var slideListItems = document.getElementById('slide-list-items');
  var editLabel = document.getElementById('edit-label');
  var contentEl = document.getElementById('content');
  var notesEl = document.getElementById('notes');
  var previewFrame = document.getElementById('preview-frame');
  var saveStatus = document.getElementById('save-status');
  var layoutSelect = document.getElementById('layout-select');
  var layoutParams = document.getElementById('layout-params');
  var slideTransition = document.getElementById('slide-transition');
  var contentArea = document.getElementById('content-area');
  var regionsArea = document.getElementById('regions-area');

  // --- WebSocket ---
  function connect() {
    var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + '//' + location.host + '/ws/edit');

    ws.onmessage = function(event) {
      var data;
      try { data = JSON.parse(event.data); } catch(e) { return; }

      if (data.type === 'init') {
        deck = data.deck;
        selectedSlide = 0;
        renderAll();
        setStatus('Connected', '');
      } else if (data.type === 'saved') {
        pendingSave = false;
        setStatus('Saved', '');
        refreshPreview();
      } else if (data.type === 'reload') {
        // Ignore reloads that are likely self-triggered from our own save
        var timeSinceSave = Date.now() - lastSaveTime;
        if (timeSinceSave < SAVE_SUPPRESS_WINDOW) {
          refreshPreview();
        } else if (!pendingSave) {
          showToast('File changed externally — reloading...');
          refreshPreview();
        }
      } else if (data.type === 'error') {
        setStatus('Error: ' + data.message, 'error');
        showToast(data.message, true);
      }
    };

    ws.onclose = function() {
      setStatus('Disconnected', 'error');
      setTimeout(connect, 1000);
    };

    ws.onerror = function() {
      ws.close();
    };
  }

  // --- Rendering ---
  function renderAll() {
    renderSlideList();
    renderEditPanel();
    renderDeckSettings();
  }

  function renderSlideList() {
    if (!deck) return;
    slideListItems.innerHTML = '';
    deck.slides.forEach(function(slide, i) {
      var card = document.createElement('div');
      card.className = 'slide-card' + (i === selectedSlide ? ' active' : '');
      var title = extractTitle(slide);
      card.innerHTML = '<span class="slide-num">' + (i + 1) + '</span>' + escapeHtml(title);
      card.onclick = function() { selectSlide(i); };
      slideListItems.appendChild(card);
    });
  }

  function extractTitle(slide) {
    var content = slide.layout ? (slide.layout.regions[0] || '') : slide.content;
    var lines = content.split('\n');
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trim();
      if (line.startsWith('#')) {
        return line.replace(/^#+\s*/, '');
      }
      if (line.length > 0) {
        return line.substring(0, 30);
      }
    }
    return '(empty)';
  }

  function renderEditPanel() {
    if (!deck || !deck.slides[selectedSlide]) return;
    var slide = deck.slides[selectedSlide];
    editLabel.textContent = 'Slide ' + (selectedSlide + 1) + ' of ' + deck.slides.length;

    // Layout
    if (slide.layout) {
      layoutSelect.value = slide.layout.kind;
      layoutParams.value = slide.layout.params;
      layoutParams.style.display = '';
      showRegions(slide.layout);
    } else {
      layoutSelect.value = 'none';
      layoutParams.value = '';
      layoutParams.style.display = 'none';
      hideRegions();
      contentEl.value = slide.content;
    }

    // Transition + centered
    slideTransition.value = slide.transition || '';
    document.getElementById('slide-centered').checked = (slide.class || '').indexOf('centered') >= 0;

    // Notes
    notesEl.value = slide.notes;
  }

  function showRegions(layout) {
    contentArea.style.display = 'none';
    regionsArea.classList.remove('hidden');
    regionsArea.innerHTML = '';

    layout.regions.forEach(function(region, i) {
      var box = document.createElement('div');
      box.className = 'region-box';
      var label = document.createElement('label');
      label.textContent = 'Region ' + (i + 1);
      var ta = document.createElement('textarea');
      ta.value = region;
      ta.oninput = function() { scheduleSave(); };
      ta.setAttribute('data-region', i);
      box.appendChild(label);
      box.appendChild(ta);
      regionsArea.appendChild(box);

      // Drag-and-drop on region textareas
      setupDragDrop(ta);
    });
  }

  function hideRegions() {
    contentArea.style.display = '';
    regionsArea.classList.add('hidden');
  }

  function renderDeckSettings() {
    if (!deck) return;
    document.getElementById('cfg-title').value = deck.config.title || '';
    document.getElementById('cfg-theme').value = deck.config.theme;
    document.getElementById('cfg-aspect').value = deck.config.aspect;
    document.getElementById('cfg-transition').value = deck.config.transition;
    document.getElementById('cfg-color').value = deck.config.color_scheme;
    document.getElementById('cfg-title-size').value = parseInt(deck.config.title_size, 10) || 67;
    document.getElementById('cfg-body-size').value = parseInt(deck.config.body_size, 10) || 32;
  }

  function refreshPreview() {
    var now = Date.now();
    if (now - lastPreviewRefresh < REFRESH_COOLDOWN) return;
    lastPreviewRefresh = now;
    previewFrame.src = '/?_t=' + Date.now() + '#/' + selectedSlide;
  }

  // --- Overflow detection ---
  var overflowWarning = document.getElementById('overflow-warning');

  function checkOverflow() {
    try {
      var doc = previewFrame.contentDocument || previewFrame.contentWindow.document;
      if (!doc) return;
      var slides = doc.querySelectorAll('.slide');
      var slide = slides[selectedSlide];
      if (!slide) return;
      var isOverflowing = slide.scrollHeight > slide.clientHeight;
      if (isOverflowing) {
        overflowWarning.classList.remove('hidden');
        contentEl.style.borderColor = '#dc2626';
      } else {
        overflowWarning.classList.add('hidden');
        contentEl.style.borderColor = '';
      }
    } catch(e) {
      // Cross-origin or not loaded
    }
  }

  previewFrame.addEventListener('load', function() {
    setTimeout(function() {
      applyFragmentReveal();
      checkOverflow();
    }, 200);
  });

  // --- Reveal-all-fragments toggle ---
  var revealAllCheckbox = document.getElementById('preview-reveal-all');
  var storedReveal = localStorage.getItem('previewRevealAll');
  revealAllCheckbox.checked = (storedReveal === null) ? true : (storedReveal === 'true');

  function applyFragmentReveal() {
    try {
      var doc = previewFrame.contentDocument || previewFrame.contentWindow.document;
      if (!doc) return;
      var frags = doc.querySelectorAll('.fragment');
      var reveal = revealAllCheckbox.checked;
      for (var i = 0; i < frags.length; i++) {
        if (reveal) frags[i].classList.add('visible');
        else frags[i].classList.remove('visible');
      }
    } catch(e) { /* cross-origin or not ready */ }
  }

  window.onPreviewRevealChange = function() {
    localStorage.setItem('previewRevealAll', revealAllCheckbox.checked ? 'true' : 'false');
    applyFragmentReveal();
  };

  // --- Sync state from DOM ---
  function syncFromDOM() {
    if (!deck || !deck.slides[selectedSlide]) return;
    var slide = deck.slides[selectedSlide];

    if (slide.layout) {
      var regionEls = regionsArea.querySelectorAll('textarea[data-region]');
      slide.layout.regions = [];
      regionEls.forEach(function(el) {
        slide.layout.regions.push(el.value);
      });
      slide.layout.kind = layoutSelect.value;
      slide.layout.params = layoutParams.value;
    } else {
      slide.content = contentEl.value;
    }

    slide.transition = slideTransition.value || null;
    slide.class = document.getElementById('slide-centered').checked ? 'centered' : null;
    slide.notes = notesEl.value;

    // Deck config
    deck.config.title = document.getElementById('cfg-title').value || null;
    deck.config.theme = document.getElementById('cfg-theme').value;
    deck.config.aspect = document.getElementById('cfg-aspect').value;
    deck.config.transition = document.getElementById('cfg-transition').value;
    deck.config.color_scheme = document.getElementById('cfg-color').value;
    deck.config.title_size = document.getElementById('cfg-title-size').value + 'px';
    deck.config.body_size = document.getElementById('cfg-body-size').value + 'px';
  }

  // --- Persistence ---
  window.scheduleSave = function() {
    if (saveTimer) clearTimeout(saveTimer);
    setStatus('Editing...', 'saving');
    saveTimer = setTimeout(sendSave, SAVE_DEBOUNCE);
  };

  function sendSave() {
    syncFromDOM();
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    pendingSave = true;
    lastSaveTime = Date.now();
    ws.send(JSON.stringify({
      type: 'save',
      deck: deck
    }));
    setStatus('Saving...', 'saving');
  }

  // --- Slide management ---
  window.selectSlide = function(index) {
    syncFromDOM();
    selectedSlide = index;
    renderSlideList();
    renderEditPanel();
    refreshPreview();
  };

  window.addSlide = function() {
    syncFromDOM();
    var newSlide = {
      content: '## New Slide\n\n',
      transition: null,
      class: null,
      notes: '',
      layout: null
    };
    deck.slides.splice(selectedSlide + 1, 0, newSlide);
    selectedSlide = selectedSlide + 1;
    renderAll();
    scheduleSave();
  };

  window.deleteSlide = function() {
    if (!deck || deck.slides.length <= 1) return;
    deck.slides.splice(selectedSlide, 1);
    if (selectedSlide >= deck.slides.length) selectedSlide = deck.slides.length - 1;
    renderAll();
    scheduleSave();
  };

  window.moveSlideUp = function() {
    if (selectedSlide <= 0) return;
    syncFromDOM();
    var tmp = deck.slides[selectedSlide];
    deck.slides[selectedSlide] = deck.slides[selectedSlide - 1];
    deck.slides[selectedSlide - 1] = tmp;
    selectedSlide--;
    renderAll();
    scheduleSave();
  };

  window.moveSlideDown = function() {
    if (!deck || selectedSlide >= deck.slides.length - 1) return;
    syncFromDOM();
    var tmp = deck.slides[selectedSlide];
    deck.slides[selectedSlide] = deck.slides[selectedSlide + 1];
    deck.slides[selectedSlide + 1] = tmp;
    selectedSlide++;
    renderAll();
    scheduleSave();
  };

  // --- Layout ---
  window.onLayoutChange = function() {
    syncFromDOM();
    var slide = deck.slides[selectedSlide];
    var kind = layoutSelect.value;

    if (kind === 'none') {
      // Move first region content to main content
      if (slide.layout && slide.layout.regions.length > 0) {
        slide.content = slide.layout.regions.join('\n\n');
      }
      slide.layout = null;
      layoutParams.style.display = 'none';
    } else {
      var defaultParams = kind === 'split' ? '50/50' : kind === 'grid' ? '2x2' : '';
      var regions = ['', ''];
      if (slide.layout) {
        regions = slide.layout.regions;
      } else if (slide.content) {
        regions = [slide.content, ''];
        slide.content = '';
      }
      slide.layout = { kind: kind, params: defaultParams, regions: regions };
      layoutParams.value = defaultParams;
      layoutParams.style.display = '';
    }

    renderEditPanel();
    scheduleSave();
  };

  window.onLayoutParamsChange = function() {
    scheduleSave();
  };

  window.onCenteredChange = function() {
    scheduleSave();
  };

  // --- Toolbar functions ---
  function getActiveTextarea() {
    var slide = deck.slides[selectedSlide];
    if (slide && slide.layout) {
      var focused = document.activeElement;
      if (focused && focused.getAttribute('data-region') !== null) return focused;
      // Default to first region
      var first = regionsArea.querySelector('textarea');
      return first || contentEl;
    }
    return contentEl;
  }

  function wrapSelection(before, after) {
    var ta = getActiveTextarea();
    var start = ta.selectionStart;
    var end = ta.selectionEnd;
    var text = ta.value;
    var selected = text.substring(start, end) || 'text';
    ta.value = text.substring(0, start) + before + selected + after + text.substring(end);
    ta.selectionStart = start + before.length;
    ta.selectionEnd = start + before.length + selected.length;
    ta.focus();
    scheduleSave();
  }

  function insertAtCursor(text) {
    var ta = getActiveTextarea();
    var start = ta.selectionStart;
    var val = ta.value;
    ta.value = val.substring(0, start) + text + val.substring(start);
    ta.selectionStart = ta.selectionEnd = start + text.length;
    ta.focus();
    scheduleSave();
  }

  function prependLine(prefix) {
    var ta = getActiveTextarea();
    var start = ta.selectionStart;
    var val = ta.value;
    // Find start of current line
    var lineStart = val.lastIndexOf('\n', start - 1) + 1;
    var lineEnd = val.indexOf('\n', start);
    if (lineEnd === -1) lineEnd = val.length;
    var line = val.substring(lineStart, lineEnd);
    // Remove existing list/heading prefix
    var cleaned = line.replace(/^(\s*)(#{1,6}\s+|[-+*]\s+|\d+\.\s+|>\s+)/, '$1');
    ta.value = val.substring(0, lineStart) + prefix + cleaned + val.substring(lineEnd);
    ta.selectionStart = ta.selectionEnd = lineStart + prefix.length + cleaned.length;
    ta.focus();
    scheduleSave();
  }

  window.toolBold = function() { wrapSelection('**', '**'); };
  window.toolItalic = function() { wrapSelection('*', '*'); };
  window.toolStrike = function() { wrapSelection('~~', '~~'); };
  window.toolHeading = function(level) { prependLine('#'.repeat(level) + ' '); };
  window.toolBulletList = function() { prependLine('- '); };
  window.toolNumberedList = function() { prependLine('1. '); };
  window.toolFragmentList = function() { prependLine('+ '); };
  window.toolBlockquote = function() { prependLine('> '); };

  window.toolCodeBlock = function() {
    insertAtCursor('\n```\ncode\n```\n');
  };

  window.toolLink = function() {
    wrapSelection('[', '](url)');
  };

  window.toolImage = function() {
    document.getElementById('file-input').click();
  };

  window.toolTable = function() {
    insertAtCursor('\n| Column A | Column B |\n|----------|----------|\n| Cell 1   | Cell 2   |\n');
  };

  // --- File upload ---
  window.handleFileSelect = function(event) {
    var file = event.target.files[0];
    if (file) uploadFile(file);
    event.target.value = '';
  };

  function uploadFile(file) {
    var formData = new FormData();
    formData.append('file', file);

    fetch('/api/upload', { method: 'POST', body: formData })
      .then(function(res) { return res.json(); })
      .then(function(data) {
        if (data.path) {
          insertAtCursor('![' + file.name + '](' + data.path + ')');
          showToast('Uploaded: ' + data.path);
        } else if (data.error) {
          showToast(data.error, true);
        }
      })
      .catch(function(err) {
        showToast('Upload failed: ' + err, true);
      });
  }

  // Drag and drop on content textarea
  function setupDragDrop(el) {
    el.addEventListener('dragover', function(e) {
      e.preventDefault();
      el.classList.add('drag-over');
    });
    el.addEventListener('dragleave', function() {
      el.classList.remove('drag-over');
    });
    el.addEventListener('drop', function(e) {
      e.preventDefault();
      el.classList.remove('drag-over');
      if (e.dataTransfer.files.length > 0) {
        uploadFile(e.dataTransfer.files[0]);
      }
    });
  }
  setupDragDrop(contentEl);

  // --- Keyboard shortcuts ---
  document.addEventListener('keydown', function(e) {
    if (e.ctrlKey || e.metaKey) {
      switch(e.key) {
        case 's':
          e.preventDefault();
          syncFromDOM();
          sendSave();
          break;
        case 'b':
          e.preventDefault();
          toolBold();
          break;
        case 'i':
          e.preventDefault();
          toolItalic();
          break;
        case 'n':
          if (e.shiftKey) {
            e.preventDefault();
            addSlide();
          }
          break;
      }
    }
  });

  // Tab inserts spaces in textareas
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Tab' && e.target.tagName === 'TEXTAREA') {
      e.preventDefault();
      var ta = e.target;
      var start = ta.selectionStart;
      ta.value = ta.value.substring(0, start) + '  ' + ta.value.substring(ta.selectionEnd);
      ta.selectionStart = ta.selectionEnd = start + 2;
      scheduleSave();
    }
  });

  // --- Deck settings auto-save ---
  document.querySelectorAll('#deck-settings input, #deck-settings select').forEach(function(el) {
    el.addEventListener('input', scheduleSave);
    el.addEventListener('change', scheduleSave);
  });

  // --- Utilities ---
  function setStatus(text, cls) {
    saveStatus.textContent = text;
    saveStatus.className = 'status' + (cls ? ' ' + cls : '');
  }

  function escapeHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function showToast(msg, isError) {
    var existing = document.querySelector('.toast');
    if (existing) existing.remove();
    var toast = document.createElement('div');
    toast.className = 'toast' + (isError ? ' error' : '');
    toast.textContent = msg;
    document.body.appendChild(toast);
    requestAnimationFrame(function() { toast.classList.add('visible'); });
    setTimeout(function() {
      toast.classList.remove('visible');
      setTimeout(function() { toast.remove(); }, 300);
    }, 2500);
  }

  // --- Preview scaling ---
  var previewPanel = document.getElementById('preview-panel');
  var previewContainer = document.getElementById('preview-container');
  var resizeHandle = document.getElementById('resize-handle');
  var SLIDE_W = 1920;
  var SLIDE_H = 1080;

  function scalePreview() {
    if (!previewContainer || previewPanel.classList.contains('collapsed')) return;
    var cw = previewContainer.clientWidth;
    var ch = previewContainer.clientHeight;
    if (cw === 0 || ch === 0) return;
    var scaleX = cw / SLIDE_W;
    var scaleY = ch / SLIDE_H;
    var scale = Math.min(scaleX, scaleY);
    previewFrame.style.transform = 'scale(' + scale + ')';
  }

  window.addEventListener('resize', scalePreview);
  new ResizeObserver(scalePreview).observe(previewContainer);

  window.togglePreview = function() {
    previewPanel.style.width = '';
    previewPanel.classList.toggle('collapsed');
    setTimeout(scalePreview, 250);
  };

  // --- Resize handle ---
  (function() {
    var dragging = false;
    var startX, startWidth;

    resizeHandle.addEventListener('mousedown', function(e) {
      dragging = true;
      startX = e.clientX;
      startWidth = previewPanel.offsetWidth;
      resizeHandle.classList.add('dragging');
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      e.preventDefault();
    });

    document.addEventListener('mousemove', function(e) {
      if (!dragging) return;
      var dx = startX - e.clientX;
      var newWidth = Math.max(200, Math.min(window.innerWidth * 0.7, startWidth + dx));
      previewPanel.style.width = newWidth + 'px';
      scalePreview();
    });

    document.addEventListener('mouseup', function() {
      if (!dragging) return;
      dragging = false;
      resizeHandle.classList.remove('dragging');
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    });
  })();

  // --- Init ---
  connect();
  setTimeout(scalePreview, 500);
})();
"##;
