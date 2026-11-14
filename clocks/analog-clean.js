// ClockScreen — Analog Clean (360x360). SVG, no numerals.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});
  var NS = 'http://www.w3.org/2000/svg';

  var els = null;
  var reduceMotion = false;

  function mount(root) {
    root.innerHTML =
      '<div class="clock-card ana-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a"></span>' +
          '<span class="blob blob-b"></span>' +
        '</div>' +
        '<div class="clock-surface ana-surface">' +
          '<svg class="ana-svg" viewBox="0 0 200 200" aria-hidden="true">' +
            '<circle class="ana-ring" cx="100" cy="100" r="95"></circle>' +
            '<g class="ana-ticks"></g>' +
            '<g class="ana-hour-pivot"><line class="ana-hand ana-hour-line" x1="100" y1="108" x2="100" y2="57"></line></g>' +
            '<g class="ana-minute-pivot"><line class="ana-hand ana-minute-line" x1="100" y1="112" x2="100" y2="37"></line></g>' +
            '<g class="ana-second-pivot"><line class="ana-hand ana-second-line" x1="100" y1="118" x2="100" y2="33"></line></g>' +
            '<circle class="ana-cap" cx="100" cy="100" r="3.2"></circle>' +
            '<circle class="ana-cap-inner" cx="100" cy="100" r="1.3"></circle>' +
          '</svg>' +
        '</div>' +
      '</div>';

    var ticks = root.querySelector('.ana-ticks');
    var frag = document.createDocumentFragment();
    for (var i = 0; i < 60; i++) {
      var major = i % 5 === 0;
      var line = document.createElementNS(NS, 'line');
      line.setAttribute('x1', '100');
      line.setAttribute('y1', String(100 - 91));
      line.setAttribute('x2', '100');
      line.setAttribute('y2', String(100 - (major ? 81 : 87)));
      line.setAttribute('transform', 'rotate(' + (i * 6) + ' 100 100)');
      line.setAttribute('class', major ? 'ana-tick ana-tick-major' : 'ana-tick');
      frag.appendChild(line);
    }
    ticks.appendChild(frag);

    els = {
      hour: root.querySelector('.ana-hour-pivot'),
      minute: root.querySelector('.ana-minute-pivot'),
      second: root.querySelector('.ana-second-pivot')
    };
    reduceMotion = !!(window.matchMedia &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  }

  function setRotation(el, deg) {
    if (el) el.setAttribute('transform', 'rotate(' + deg.toFixed(2) + ' 100 100)');
  }

  function update(now) {
    if (!els) return;
    var ms = reduceMotion ? 0 : now.getMilliseconds();
    var sec = now.getSeconds() + ms / 1000;
    var min = now.getMinutes() + sec / 60;
    var hr = (now.getHours() % 12) + min / 60;

    setRotation(els.hour, hr * 30);
    setRotation(els.minute, min * 6);
    setRotation(els.second, sec * 6);
  }

  function unmount() {
    els = null;
  }

  CLOCKS.analog = {
    id: 'analog',
    name: 'Analog Clean',
    smooth: true, // informs scheduler to run smooth 60fps sweep
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
