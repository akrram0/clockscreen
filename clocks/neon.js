// ClockScreen — Cyberpunk Neon (440x210).
// Retro-futuristic cyberpunk / synthwave desktop HUD clock.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  function pad2(n) {
    return n < 10 ? '0' + n : String(n);
  }

  var els = null;
  var lastTimeKey = '';

  function mount(root) {
    root.innerHTML =
      '<div class="clock-card neon-card">' +
        '<div class="neon-grid-bg" aria-hidden="true"></div>' +
        '<div class="clock-surface neon-surface">' +
          '<div class="neon-header">' +
            '<span class="neon-tag">SYS // CLK</span>' +
            '<span class="neon-sec">SEC: 00</span>' +
          '</div>' +
          '<div class="neon-main">' +
            '<span class="neon-digits">00:00</span>' +
            '<span class="neon-ampm" hidden>AM</span>' +
          '</div>' +
          '<div class="neon-progress-track" aria-hidden="true">' +
            '<div class="neon-progress-bar"></div>' +
          '</div>' +
        '</div>' +
      '</div>';

    els = {
      digits: root.querySelector('.neon-digits'),
      ampm: root.querySelector('.neon-ampm'),
      sec: root.querySelector('.neon-sec'),
      bar: root.querySelector('.neon-progress-bar')
    };
    lastTimeKey = '';
  }

  function update(now, api) {
    if (!els) return;
    var state = (api && api.state) || {};
    var hour12 = !!state.hour12;

    var h = now.getHours();
    var m = now.getMinutes();
    var s = now.getSeconds();

    var timeStr;
    var suffix = '';
    if (hour12) {
      timeStr = pad2(h % 12 || 12) + ':' + pad2(m);
      suffix = h < 12 ? 'AM' : 'PM';
    } else {
      timeStr = pad2(h) + ':' + pad2(m);
    }

    var timeKey = timeStr + '|' + suffix;
    if (timeKey !== lastTimeKey) {
      els.digits.textContent = timeStr;
      if (hour12) {
        els.ampm.hidden = false;
        els.ampm.textContent = suffix;
      } else {
        els.ampm.hidden = true;
      }
      lastTimeKey = timeKey;
    }

    els.sec.textContent = 'SEC: ' + pad2(s);
    var pct = Math.round((s / 59) * 100);
    els.bar.style.width = pct + '%';
  }

  function unmount() {
    els = null;
    lastTimeKey = '';
  }

  CLOCKS.neon = {
    id: 'neon',
    name: 'Cyberpunk Neon',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
