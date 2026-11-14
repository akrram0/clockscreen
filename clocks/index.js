// ClockScreen — renderer controller. Classic script, no modules.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});
  var appEl = document.getElementById('app');
  var rootStyle = document.documentElement.style;

  var DEFAULT_STATE = {
    style: 'minimal',
    scale: 1,
    opacity: 1,
    locked: false,
    alwaysOnTop: false,
    clickThrough: false,
    hour12: false,
    locale: 'auto',
    arabicDate: false,
    base: { width: 420, height: 200 }
  };

  var FADE_OUT = 180;
  var FADE_IN = 260;

  var state = {
    style: DEFAULT_STATE.style,
    scale: DEFAULT_STATE.scale,
    opacity: DEFAULT_STATE.opacity,
    locked: DEFAULT_STATE.locked,
    alwaysOnTop: DEFAULT_STATE.alwaysOnTop,
    clickThrough: DEFAULT_STATE.clickThrough,
    hour12: DEFAULT_STATE.hour12,
    locale: DEFAULT_STATE.locale,
    arabicDate: DEFAULT_STATE.arabicDate,
    base: { width: DEFAULT_STATE.base.width, height: DEFAULT_STATE.base.height }
  };

  var rawApi = window.clockAPI || null;
  var unsubscribe = null;
  var booted = false;
  var active = null;
  var stageEl = null;
  var switchTimer = null;
  var rafId = 0;
  var timerId = null;
  var dragging = false;
  var lastWheelAt = 0;
  var updateErrorId = null;

  var reduceMotion = !!(window.matchMedia &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches);

  function noop() {}

  // Snapshot API handed to clocks: getState() is synchronous here so update()
  // never has to await IPC.
  var clockApi = {
    get state() { return state; },
    getState: function () { return state; },
    getLocale: function () {
      if (state.locale === 'ar' || state.arabicDate) return 'ar-EG';
      if (state.locale === 'en') return 'en-US';
      if (state.locale && state.locale !== 'auto') return state.locale;
      return (typeof navigator !== 'undefined' && navigator.language) ? navigator.language : 'en-US';
    },
    onState: function (cb) {
      return rawApi && typeof rawApi.onState === 'function'
        ? rawApi.onState(cb)
        : noop;
    },
    dragStart: function () {
      if (rawApi && typeof rawApi.dragStart === 'function') rawApi.dragStart();
    },
    dragEnd: function () {
      if (rawApi && typeof rawApi.dragEnd === 'function') rawApi.dragEnd();
    },
    scaleBy: function (dir) {
      if (rawApi && typeof rawApi.scaleBy === 'function') rawApi.scaleBy(dir);
    },
    setOpacity: function (val) {
      if (rawApi && typeof rawApi.setOpacity === 'function') rawApi.setOpacity(val);
    },
    openMenu: function () {
      if (rawApi && typeof rawApi.openMenu === 'function') rawApi.openMenu();
    }
  };

  function applyScale() {
    var s = Number(state.scale);
    if (!isFinite(s) || s <= 0) s = 1;
    rootStyle.setProperty('--scale', String(s));
  }

  function fadeMs(ms) {
    return reduceMotion ? 0 : ms;
  }

  // ---------- adaptive scheduler (sub-second RAF vs second-boundary setTimeout) ----------
  function stopTicker() {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
    if (timerId) {
      clearTimeout(timerId);
      timerId = null;
    }
  }

  function runTick() {
    if (!active || !stageEl) return;
    var now = new Date();
    try {
      active.update(now, clockApi);
    } catch (err) {
      if (updateErrorId !== active.id) {
        updateErrorId = active.id;
        console.error('ClockScreen: update failed for ' + active.id, err);
      }
    }
  }

  function rafLoop() {
    runTick();
    if (active && active.smooth && !reduceMotion) {
      rafId = requestAnimationFrame(rafLoop);
    }
  }

  function timerLoop() {
    runTick();
    if (!active || (active.smooth && !reduceMotion)) return;
    var now = new Date();
    var ms = 1000 - now.getMilliseconds();
    if (ms <= 10) ms = 1000;
    timerId = setTimeout(timerLoop, ms);
  }

  function restartTicker() {
    stopTicker();
    if (!active || !stageEl) return;

    if (active.smooth && !reduceMotion) {
      rafId = requestAnimationFrame(rafLoop);
    } else {
      timerLoop();
    }
  }

  function mountClock(id) {
    var def = CLOCKS[id] || CLOCKS.minimal;
    if (!def) {
      console.error('ClockScreen: no clock registered for "' + id + '"');
      return;
    }

    var el = document.createElement('div');
    el.className = 'stage';
    el.dataset.clock = def.id || id;
    el.style.opacity = '0';
    el.style.transition = 'none';

    stageEl = el;
    active = def;
    updateErrorId = null;
    appEl.appendChild(el);

    try {
      if (typeof def.mount === 'function') def.mount(el, clockApi);
    } catch (err) {
      console.error('ClockScreen: mount failed for ' + id, err);
    }

    // Flush styles so the fade-in starts smoothly from opacity 0.
    void el.offsetWidth;
    var dur = fadeMs(FADE_IN);
    el.style.transition = dur ? 'opacity ' + dur + 'ms ease' : 'none';
    el.style.opacity = '1';

    restartTicker();
  }

  function unmountActive() {
    stopTicker();
    if (!active || !stageEl) return;
    var def = active;
    var el = stageEl;
    stageEl = null;
    active = null;
    try {
      if (typeof def.unmount === 'function') def.unmount();
    } catch (err) {
      console.error('ClockScreen: unmount failed for ' + def.id, err);
    }
    if (el.parentNode) el.parentNode.removeChild(el);
  }

  function switchClock(id) {
    var wasSwitching = !!switchTimer;
    if (switchTimer) {
      clearTimeout(switchTimer);
      switchTimer = null;
    }

    if (active && active.id === id) {
      if (wasSwitching && stageEl) {
        var inDur = fadeMs(FADE_IN);
        stageEl.style.transition = inDur ? 'opacity ' + inDur + 'ms ease' : 'none';
        stageEl.style.opacity = '1';
      }
      return;
    }

    if (!active || !stageEl) {
      mountClock(id);
      return;
    }

    var oldEl = stageEl;
    var oldDef = active;
    var dur = fadeMs(FADE_OUT);
    oldEl.style.transition = dur ? 'opacity ' + dur + 'ms ease' : 'none';
    oldEl.style.opacity = '0';

    function finish() {
      switchTimer = null;
      stopTicker();
      try {
        if (typeof oldDef.unmount === 'function') oldDef.unmount();
      } catch (err) {
        console.error('ClockScreen: unmount failed for ' + oldDef.id, err);
      }
      if (oldEl.parentNode) oldEl.parentNode.removeChild(oldEl);
      if (stageEl === oldEl) {
        stageEl = null;
        active = null;
      }
      mountClock(id);
    }

    if (dur === 0) finish();
    else switchTimer = setTimeout(finish, dur);
  }

  function applyState(next) {
    if (!next || typeof next !== 'object') return;

    var style = typeof next.style === 'string' && CLOCKS[next.style]
      ? next.style
      : 'minimal';

    var scale = Number(next.scale);
    if (!isFinite(scale) || scale <= 0) scale = state.scale;

    var base = state.base;
    if (next.base && isFinite(Number(next.base.width)) && isFinite(Number(next.base.height))) {
      base = { width: Number(next.base.width), height: Number(next.base.height) };
    }

    state = {
      style: style,
      scale: scale,
      opacity: next.opacity !== undefined ? Number(next.opacity) : state.opacity,
      locked: !!next.locked,
      alwaysOnTop: !!next.alwaysOnTop,
      clickThrough: !!next.clickThrough,
      hour12: !!next.hour12,
      locale: next.locale || (next.arabicDate ? 'ar' : 'auto'),
      arabicDate: !!next.arabicDate,
      base: base
    };

    applyScale();

    if (!booted) {
      booted = true;
      mountClock(state.style);
    } else {
      if (active && active.id !== state.style) {
        switchClock(state.style);
      } else {
        runTick();
      }
    }
  }

  // ---------- input ----------
  function onWheel(e) {
    e.preventDefault();
    var t = window.performance && performance.now ? performance.now() : Date.now();
    if (t - lastWheelAt < 80) return;
    lastWheelAt = t;
    clockApi.scaleBy(e.deltaY < 0 ? 1 : -1);
  }

  function onContextMenu(e) {
    e.preventDefault();
    clockApi.openMenu();
  }

  function onPointerDown(e) {
    if (e.button !== 0) return;
    if (e.isPrimary === false) return;
    if (state.locked || state.clickThrough || dragging) return;

    dragging = true;
    try {
      if (appEl.setPointerCapture) appEl.setPointerCapture(e.pointerId);
    } catch (err) {}
    clockApi.dragStart();
  }

  function onPointerUp(e) {
    if (!dragging) return;
    try {
      if (appEl.hasPointerCapture && appEl.hasPointerCapture(e.pointerId)) {
        appEl.releasePointerCapture(e.pointerId);
      }
    } catch (err) {}
    dragging = false;
    clockApi.dragEnd();
  }

  function onLostCapture() {
    if (!dragging) return;
    dragging = false;
    clockApi.dragEnd();
  }

  function onBlur() {
    if (!dragging) return;
    dragging = false;
    clockApi.dragEnd();
  }

  function onDragStart(e) {
    e.preventDefault();
  }

  // ---------- lifecycle ----------
  function cleanup() {
    if (dragging) {
      dragging = false;
      clockApi.dragEnd();
    }
    if (switchTimer) {
      clearTimeout(switchTimer);
      switchTimer = null;
    }
    stopTicker();
    if (typeof unsubscribe === 'function') {
      try { unsubscribe(); } catch (err) {}
      unsubscribe = null;
    }
    unmountActive();
    window.removeEventListener('wheel', onWheel, { passive: false });
    window.removeEventListener('contextmenu', onContextMenu);
    window.removeEventListener('pointerdown', onPointerDown);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('pointercancel', onPointerUp);
    window.removeEventListener('lostpointercapture', onLostCapture);
    window.removeEventListener('blur', onBlur);
    window.removeEventListener('dragstart', onDragStart);
  }

  function boot() {
    if (rawApi && typeof rawApi.onState === 'function') {
      try {
        unsubscribe = rawApi.onState(function (s) { applyState(s); });
      } catch (err) {
        console.error('ClockScreen: could not subscribe to state', err);
      }
    }

    if (rawApi && typeof rawApi.getState === 'function') {
      var result = null;
      try {
        result = rawApi.getState();
      } catch (err) {
        console.error('ClockScreen: getState failed', err);
      }

      if (result && typeof result.then === 'function') {
        result.then(function (s) {
          if (!booted) applyState(s);
        }).catch(function (err) {
          console.error('ClockScreen: getState rejected', err);
          if (!booted) applyState(DEFAULT_STATE);
        });
      } else if (result) {
        if (!booted) applyState(result);
      } else if (!booted) {
        applyState(DEFAULT_STATE);
      }
    } else if (!booted) {
      applyState(DEFAULT_STATE);
    }
  }

  window.addEventListener('wheel', onWheel, { passive: false });
  window.addEventListener('contextmenu', onContextMenu);
  window.addEventListener('pointerdown', onPointerDown);
  window.addEventListener('pointerup', onPointerUp);
  window.addEventListener('pointercancel', onPointerUp);
  window.addEventListener('lostpointercapture', onLostCapture);
  window.addEventListener('blur', onBlur);
  window.addEventListener('dragstart', onDragStart);
  window.addEventListener('pagehide', cleanup);

  boot();
})();
