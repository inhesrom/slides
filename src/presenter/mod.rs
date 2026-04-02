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
      <div class="slide-container" id="current-container">
        <iframe id="current-slide" class="slide-frame"></iframe>
      </div>
    </div>
    <div class="side-panel">
      <div class="slide-container next-container" id="next-container">
        <iframe id="next-slide" class="slide-frame"></iframe>
      </div>
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
html, body { height: 100%; overflow: hidden; }
body { background: #1a1a2e; color: #e2e8f0; font-family: system-ui, sans-serif; }
.presenter { display: grid; grid-template-columns: 2fr 1fr; height: 100vh; gap: 1rem; padding: 1rem; }
.main-panel { display: flex; align-items: center; justify-content: center; }
.side-panel { display: flex; flex-direction: column; gap: 1rem; min-height: 0; }

/* Slide containers scale the full-size iframe to fit */
.slide-container { position: relative; overflow: hidden; border: 2px solid #334155; border-radius: 8px; width: 100%; aspect-ratio: 16/9; }
.main-panel .slide-container { height: 100%; width: auto; aspect-ratio: 16/9; max-width: 100%; }
.next-container { opacity: 0.8; }
.slide-label { position: absolute; top: 0.4rem; left: 0.6rem; font-size: 0.65rem; color: #94a3b8; background: rgba(15,23,42,0.7); padding: 0.15rem 0.4rem; border-radius: 3px; z-index: 2; text-transform: uppercase; letter-spacing: 0.05em; }

/* Iframes render at full presentation size, scaled down */
.slide-frame { position: absolute; top: 0; left: 0; width: 1920px; height: 1080px; border: none; transform-origin: top left; }

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
  var currentFragments = 0; // how many fragments are currently visible
  var totalSlides = 0;
  var slidesData = []; // [{notes, fragmentCount}, ...]
  var timerStart = Date.now();
  var timerRunning = true;
  var timerOffset = 0;

  var currentFrame = document.getElementById('current-slide');
  var nextFrame = document.getElementById('next-slide');
  var currentContainer = document.getElementById('current-container');
  var nextContainer = document.getElementById('next-container');
  var notesEl = document.getElementById('notes-content');
  var timerEl = document.getElementById('timer');
  var progressEl = document.getElementById('progress');
  var timerBtn = document.getElementById('timer-btn');

  var SLIDE_W = 1920;
  var SLIDE_H = 1080;
  var presenterSyncId = Math.random().toString(36).substr(2, 9);

  // Scale iframes to fit their containers
  function scaleFrames() {
    scaleFrame(currentFrame, currentContainer);
    scaleFrame(nextFrame, nextContainer);
  }

  function scaleFrame(frame, container) {
    var cw = container.clientWidth;
    var ch = container.clientHeight;
    if (cw === 0 || ch === 0) return;
    var scale = Math.min(cw / SLIDE_W, ch / SLIDE_H);
    frame.style.transform = 'scale(' + scale + ')';
  }

  window.addEventListener('resize', scaleFrames);
  new ResizeObserver(scaleFrames).observe(currentContainer);
  new ResizeObserver(scaleFrames).observe(nextContainer);

  // Connect to the server for reload/navigate events
  var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var ws = new WebSocket(protocol + '//' + location.host + '/ws');

  ws.onmessage = function(event) {
    var data;
    try { data = JSON.parse(event.data); } catch(e) { return; }
    if (data.type === 'reload') {
      currentFrame.src = '/';
    } else if (data.type === 'sync' && data.origin !== presenterSyncId) {
      // Incoming sync from audience view — follow their navigation
      currentSlide = data.slide;
      currentFragments = typeof data.fragments === 'number' ? data.fragments : 0;
      updatePresenterView();
    }
  };

  ws.onclose = function() {
    setTimeout(function() { location.reload(); }, 1000);
  };

  // Load the deck in both iframes
  currentFrame.src = '/';
  nextFrame.src = '/';

  // Once the current iframe loads, extract slide data and update view
  var currentLoaded = false;
  var nextLoaded = false;

  currentFrame.addEventListener('load', function() {
    currentLoaded = true;
    extractSlideData();
    scaleFrames();
    if (nextLoaded) updatePresenterView();
  });

  nextFrame.addEventListener('load', function() {
    nextLoaded = true;
    scaleFrames();
    if (currentLoaded) updatePresenterView();
  });

  function extractSlideData() {
    try {
      var doc = currentFrame.contentDocument || currentFrame.contentWindow.document;
      var slides = doc.querySelectorAll('.slide');
      totalSlides = slides.length;
      slidesData = [];
      for (var i = 0; i < slides.length; i++) {
        slidesData.push({
          notes: slides[i].getAttribute('data-notes') || '',
          fragmentCount: slides[i].querySelectorAll('.fragment').length
        });
      }
    } catch(e) {
      // Cross-origin or not loaded yet
    }
  }

  function updatePresenterView() {
    // Navigate the current iframe to the right slide with current fragment state
    if (currentFrame.contentWindow) {
      currentFrame.contentWindow.postMessage({
        type: 'goto', slide: currentSlide, fragments: currentFragments
      }, '*');
    }

    // Show next preview: what happens on the next keypress?
    updateNextPreview();

    // Update notes
    if (slidesData[currentSlide] && slidesData[currentSlide].notes) {
      notesEl.textContent = slidesData[currentSlide].notes;
    } else {
      notesEl.textContent = 'No notes for this slide.';
    }

    // Update progress
    var fragInfo = '';
    var sd = slidesData[currentSlide];
    if (sd && sd.fragmentCount > 0) {
      fragInfo = ' (fragment ' + currentFragments + '/' + sd.fragmentCount + ')';
    }
    progressEl.textContent = 'Slide ' + (currentSlide + 1) + ' / ' + totalSlides + fragInfo;
  }

  function updateNextPreview() {
    var sd = slidesData[currentSlide];
    var hasMoreFragments = sd && currentFragments < sd.fragmentCount;

    if (hasMoreFragments) {
      // Next action reveals another fragment on the same slide
      // Show current slide with one more fragment revealed
      if (nextFrame.contentWindow) {
        nextFrame.contentWindow.postMessage({
          type: 'goto', slide: currentSlide, fragments: currentFragments + 1
        }, '*');
      }
    } else if (currentSlide + 1 < totalSlides) {
      // Next action goes to the next slide
      if (nextFrame.contentWindow) {
        nextFrame.contentWindow.postMessage({
          type: 'goto', slide: currentSlide + 1, fragments: 0
        }, '*');
      }
    }
  }

  // Keyboard navigation
  document.addEventListener('keydown', function(e) {
    switch(e.key) {
      case 'ArrowRight':
      case 'ArrowDown':
      case ' ':
      case 'PageDown':
        e.preventDefault();
        navigateForward();
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
      case 'PageUp':
        e.preventDefault();
        navigateBackward();
        break;
    }
  });

  function navigateForward() {
    var sd = slidesData[currentSlide];
    if (sd && currentFragments < sd.fragmentCount) {
      currentFragments++;
    } else if (currentSlide + 1 < totalSlides) {
      currentSlide++;
      currentFragments = 0;
    } else {
      return;
    }
    updatePresenterView();
    broadcastSync();
  }

  function navigateBackward() {
    if (currentFragments > 0) {
      currentFragments--;
    } else if (currentSlide > 0) {
      currentSlide--;
      var sd = slidesData[currentSlide];
      currentFragments = sd ? sd.fragmentCount : 0;
    } else {
      return;
    }
    updatePresenterView();
    broadcastSync();
  }

  function broadcastSync() {
    ws.send(JSON.stringify({
      type: 'sync',
      slide: currentSlide,
      fragments: currentFragments,
      origin: presenterSyncId
    }));
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
  setTimeout(scaleFrames, 500);
})();
"#;
