(function() {
  'use strict';

  var current = 0;
  var slides = document.querySelectorAll('.slide');
  var total = slides.length;

  function showSlide(index) {
    if (index < 0 || index >= total) return;
    slides[current].classList.remove('active');
    slides[current].classList.remove('enter-forward', 'enter-backward');
    var direction = index > current ? 'forward' : 'backward';
    current = index;
    slides[current].classList.add('active');
    slides[current].classList.add('enter-' + direction);
    updateProgress();
    updateHash();
    notifyParent();
  }

  function notifyParent() {
    if (window.parent === window) return;
    var frags = getFragments();
    var visibleCount = 0;
    for (var i = 0; i < frags.length; i++) {
      if (frags[i].classList.contains('visible')) visibleCount++;
    }
    window.parent.postMessage({
      type: 'preview-slide-change',
      slide: current,
      fragments: visibleCount
    }, '*');
  }

  function getFragments() {
    var slide = slides[current];
    if (!slide) return [];
    return Array.prototype.slice.call(slide.querySelectorAll('.fragment'));
  }

  function nextVisibleFragment() {
    var frags = getFragments();
    for (var i = 0; i < frags.length; i++) {
      if (!frags[i].classList.contains('visible')) return i;
    }
    return -1;
  }

  function prevVisibleFragment() {
    var frags = getFragments();
    for (var i = frags.length - 1; i >= 0; i--) {
      if (frags[i].classList.contains('visible')) return i;
    }
    return -1;
  }

  function next() {
    var idx = nextVisibleFragment();
    if (idx >= 0) {
      getFragments()[idx].classList.add('visible');
    } else {
      showSlide(current + 1);
    }
  }

  function prev() {
    var idx = prevVisibleFragment();
    if (idx >= 0) {
      getFragments()[idx].classList.remove('visible');
    } else {
      showSlide(current - 1);
    }
  }

  function updateProgress() {
    var fill = document.getElementById('progress-fill');
    if (fill) {
      fill.style.width = ((current + 1) / total * 100) + '%';
    }
  }

  function updateHash() {
    history.replaceState(null, '', '#/' + current);
  }

  function readHash() {
    var match = location.hash.match(/^#\/(\d+)$/);
    if (match) {
      var idx = parseInt(match[1], 10);
      if (idx >= 0 && idx < total) return idx;
    }
    return 0;
  }

  // Keyboard navigation
  document.addEventListener('keydown', function(e) {
    switch(e.key) {
      case 'ArrowRight':
      case 'ArrowDown':
      case ' ':
      case 'PageDown':
        e.preventDefault();
        next();
        broadcastSync();
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
      case 'PageUp':
        e.preventDefault();
        prev();
        broadcastSync();
        break;
      case 'Home':
        e.preventDefault();
        showSlide(0);
        broadcastSync();
        break;
      case 'End':
        e.preventDefault();
        showSlide(total - 1);
        broadcastSync();
        break;
      case 'f':
      case 'F':
        e.preventDefault();
        toggleFullscreen();
        break;
      case 'd':
      case 'D':
        e.preventDefault();
        toggleDarkMode();
        break;
      case 'p':
      case 'P':
        e.preventDefault();
        window.open('/presenter', '_blank');
        break;
      case 'Escape':
        if (document.fullscreenElement) {
          document.exitFullscreen();
        }
        break;
    }
  });

  function toggleFullscreen() {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen();
    } else {
      document.exitFullscreen();
    }
  }

  function toggleDarkMode() {
    var root = document.documentElement;
    if (root.classList.contains('dark')) {
      root.classList.remove('dark');
      root.classList.add('light');
    } else {
      root.classList.remove('light');
      root.classList.add('dark');
    }
  }

  // Touch support
  var touchStartX = 0;
  document.addEventListener('touchstart', function(e) {
    touchStartX = e.changedTouches[0].screenX;
  });
  document.addEventListener('touchend', function(e) {
    var dx = e.changedTouches[0].screenX - touchStartX;
    if (Math.abs(dx) > 50) {
      if (dx > 0) prev(); else next();
      broadcastSync();
    }
  });

  // Hash navigation
  window.addEventListener('hashchange', function() {
    showSlide(readHash());
  });

  // WebSocket for live reload and sync
  // Skip WebSocket when embedded in an iframe (presenter controls us via postMessage)
  var isEmbedded = window.parent !== window;
  var ws = null;
  var syncId = Math.random().toString(36).substr(2, 9);

  function connectWS() {
    if (isEmbedded) return;

    var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + '//' + location.host + '/ws');

    ws.onmessage = function(event) {
      var data;
      try { data = JSON.parse(event.data); } catch(e) { return; }
      if (data.type === 'reload') {
        location.reload();
      } else if (data.type === 'navigate') {
        showSlide(data.slide);
      } else if (data.type === 'sync' && data.origin !== syncId) {
        // Sync from presenter or another view — jump to exact state
        showSlide(data.slide);
        var frags = getFragments();
        var count = typeof data.fragments === 'number' ? data.fragments : 0;
        for (var fi = 0; fi < frags.length; fi++) {
          if (fi < count) {
            frags[fi].classList.add('visible');
          } else {
            frags[fi].classList.remove('visible');
          }
        }
      }
    };

    ws.onclose = function() {
      setTimeout(connectWS, 1000);
    };

    ws.onerror = function() {
      ws.close();
    };
  }

  // Broadcast current state after local navigation
  function broadcastSync() {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    var frags = getFragments();
    var visibleCount = 0;
    for (var i = 0; i < frags.length; i++) {
      if (frags[i].classList.contains('visible')) visibleCount++;
    }
    ws.send(JSON.stringify({
      type: 'sync',
      slide: current,
      fragments: visibleCount,
      origin: syncId
    }));
  }

  // Listen for postMessage from presenter view
  window.addEventListener('message', function(e) {
    if (!e.data) return;
    if (e.data.type === 'goto' && typeof e.data.slide === 'number') {
      showSlide(e.data.slide);
      // Optionally reveal a specific number of fragments
      if (typeof e.data.fragments === 'number') {
        var frags = getFragments();
        for (var i = 0; i < frags.length; i++) {
          if (i < e.data.fragments) {
            frags[i].classList.add('visible');
          } else {
            frags[i].classList.remove('visible');
          }
        }
      }
    } else if (e.data.type === 'update-slide'
               && typeof e.data.index === 'number'
               && typeof e.data.html === 'string') {
      // In-place replace of a single slide's <section>, sent by the editor
      // on every keystroke. Preserve the .active marker and any revealed
      // fragments so mid-edit updates don't flash the slide back to hidden.
      var idx = e.data.index;
      var oldEl = slides[idx];
      if (!oldEl) return;
      var wrapper = document.createElement('div');
      wrapper.innerHTML = e.data.html;
      var newEl = wrapper.firstElementChild;
      if (!newEl) return;
      if (oldEl.classList.contains('active')) newEl.classList.add('active');
      var oldFrags = oldEl.querySelectorAll('.fragment');
      var visibleCount = 0;
      for (var i = 0; i < oldFrags.length; i++) {
        if (oldFrags[i].classList.contains('visible')) visibleCount++;
      }
      var newFrags = newEl.querySelectorAll('.fragment');
      for (var j = 0; j < newFrags.length && j < visibleCount; j++) {
        newFrags[j].classList.add('visible');
      }
      oldEl.parentNode.replaceChild(newEl, oldEl);
      slides = document.querySelectorAll('.slide');
    }
  });

  // Initialize
  current = readHash();
  if (slides[current]) {
    slides[current].classList.add('active');
  }
  updateProgress();
  connectWS();
})();
