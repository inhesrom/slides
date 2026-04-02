/// Generate the presenter view HTML.
/// Shows current slide, next slide preview, speaker notes, timer, and progress.
pub fn presenter_html(deck_title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>{title} — Presenter</title>
  <style>{css}</style>
</head>
<body>
  <div class="presenter">
    <div class="main-panel">
      <iframe id="current-slide" class="slide-frame current"></iframe>
    </div>
    <div class="side-panel">
      <iframe id="next-slide" class="slide-frame next"></iframe>
      <div class="info-panel">
        <div class="notes" id="notes">
          <h3>Speaker Notes</h3>
          <div id="notes-content">No notes for this slide.</div>
        </div>
        <div class="controls">
          <div class="timer" id="timer">00:00:00</div>
          <div class="progress" id="progress">Slide 1 / ?</div>
          <div class="timer-controls">
            <button onclick="resetTimer()">Reset</button>
            <button onclick="toggleTimer()" id="timer-btn">Pause</button>
          </div>
        </div>
      </div>
    </div>
  </div>
  <script>{js}</script>
</body>
</html>"#,
        title = deck_title,
        css = PRESENTER_CSS,
        js = PRESENTER_JS,
    )
}

const PRESENTER_CSS: &str = r#"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { background: #1a1a2e; color: #e2e8f0; font-family: system-ui, sans-serif; }
.presenter { display: grid; grid-template-columns: 2fr 1fr; height: 100vh; gap: 1rem; padding: 1rem; }
.main-panel { display: flex; align-items: center; justify-content: center; }
.side-panel { display: flex; flex-direction: column; gap: 1rem; }
.slide-frame { border: 2px solid #334155; border-radius: 8px; background: white; width: 100%; }
.slide-frame.current { height: 100%; aspect-ratio: 16/9; }
.slide-frame.next { height: 40%; aspect-ratio: 16/9; opacity: 0.8; }
.info-panel { flex: 1; display: flex; flex-direction: column; gap: 1rem; }
.notes { flex: 1; background: #0f172a; border-radius: 8px; padding: 1.5rem; overflow-y: auto; }
.notes h3 { font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.05em; color: #94a3b8; margin-bottom: 0.75rem; }
#notes-content { font-size: 1.1rem; line-height: 1.6; white-space: pre-wrap; }
.controls { display: flex; flex-wrap: wrap; gap: 1rem; align-items: center; justify-content: space-between; padding: 1rem; background: #0f172a; border-radius: 8px; }
.timer { font-size: 2.5rem; font-weight: 700; font-variant-numeric: tabular-nums; }
.progress { font-size: 1.1rem; color: #94a3b8; }
.timer-controls { display: flex; gap: 0.5rem; }
.timer-controls button { background: #334155; color: #e2e8f0; border: none; padding: 0.5rem 1rem; border-radius: 4px; cursor: pointer; font-size: 0.9rem; }
.timer-controls button:hover { background: #475569; }
"#;

const PRESENTER_JS: &str = r#"
(function() {
  var currentSlide = 0;
  var totalSlides = 0;
  var slidesData = [];
  var timerStart = Date.now();
  var timerRunning = true;
  var timerOffset = 0;

  var currentFrame = document.getElementById('current-slide');
  var nextFrame = document.getElementById('next-slide');
  var notesEl = document.getElementById('notes-content');
  var timerEl = document.getElementById('timer');
  var progressEl = document.getElementById('progress');
  var timerBtn = document.getElementById('timer-btn');

  // Connect to the server for reload/navigate events
  var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var ws = new WebSocket(protocol + '//' + location.host + '/ws');

  ws.onmessage = function(event) {
    var data;
    try { data = JSON.parse(event.data); } catch(e) { return; }
    if (data.type === 'reload') {
      currentFrame.src = '/';
    } else if (data.type === 'navigate') {
      currentSlide = data.slide;
      updatePresenterView();
    }
  };

  ws.onclose = function() {
    setTimeout(function() { location.reload(); }, 1000);
  };

  // Load the main deck in the current iframe
  currentFrame.src = '/';

  // Once the iframe loads, extract slide data (notes, count) from its DOM
  currentFrame.addEventListener('load', function() {
    extractSlideData();
    updatePresenterView();
  });

  function extractSlideData() {
    try {
      var doc = currentFrame.contentDocument || currentFrame.contentWindow.document;
      var slides = doc.querySelectorAll('.slide');
      totalSlides = slides.length;
      slidesData = [];
      for (var i = 0; i < slides.length; i++) {
        slidesData.push({
          notes: slides[i].getAttribute('data-notes') || ''
        });
      }
    } catch(e) {
      // Cross-origin or not loaded yet
    }
  }

  function updatePresenterView() {
    // Navigate the current iframe to the right slide
    if (currentFrame.contentWindow) {
      currentFrame.contentWindow.postMessage({ type: 'goto', slide: currentSlide }, '*');
    }

    // Load next slide preview
    if (currentSlide + 1 < totalSlides) {
      nextFrame.src = '/#/' + (currentSlide + 1);
    }

    // Update notes
    if (slidesData[currentSlide] && slidesData[currentSlide].notes) {
      notesEl.textContent = slidesData[currentSlide].notes;
    } else {
      notesEl.textContent = 'No notes for this slide.';
    }

    // Update progress
    progressEl.textContent = 'Slide ' + (currentSlide + 1) + ' / ' + totalSlides;
  }

  // Keyboard navigation
  document.addEventListener('keydown', function(e) {
    switch(e.key) {
      case 'ArrowRight':
      case 'ArrowDown':
      case ' ':
      case 'PageDown':
        e.preventDefault();
        navigate(1);
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
      case 'PageUp':
        e.preventDefault();
        navigate(-1);
        break;
    }
  });

  function navigate(delta) {
    var newIdx = currentSlide + delta;
    if (newIdx >= 0 && newIdx < totalSlides) {
      currentSlide = newIdx;
      updatePresenterView();
      ws.send(JSON.stringify({ type: 'navigate', slide: currentSlide }));
    }
  }

  // Timer
  function updateTimer() {
    if (!timerRunning) return;
    var elapsed = Date.now() - timerStart + timerOffset;
    var seconds = Math.floor(elapsed / 1000);
    var h = Math.floor(seconds / 3600);
    var m = Math.floor((seconds % 3600) / 60);
    var s = seconds % 60;
    timerEl.textContent =
      String(h).padStart(2, '0') + ':' +
      String(m).padStart(2, '0') + ':' +
      String(s).padStart(2, '0');
    requestAnimationFrame(updateTimer);
  }

  window.resetTimer = function() {
    timerStart = Date.now();
    timerOffset = 0;
    if (!timerRunning) {
      timerEl.textContent = '00:00:00';
    }
  };

  window.toggleTimer = function() {
    if (timerRunning) {
      timerRunning = false;
      timerOffset += Date.now() - timerStart;
      timerBtn.textContent = 'Resume';
    } else {
      timerRunning = true;
      timerStart = Date.now();
      timerBtn.textContent = 'Pause';
      requestAnimationFrame(updateTimer);
    }
  };

  requestAnimationFrame(updateTimer);
})();
"#;
