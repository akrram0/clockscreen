// ClockScreen — Dot Matrix (420x200). Canvas 2D, 5x7 LED font.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  var GLYPHS = {
    '0': ['01110', '10001', '10011', '10101', '11001', '10001', '01110'],
    '1': ['00100', '01100', '00100', '00100', '00100', '00100', '01110'],
    '2': ['01110', '10001', '00001', '00010', '00100', '01000', '11111'],
    '3': ['11111', '00010', '00100', '00010', '00001', '10001', '01110'],
    '4': ['00010', '00110', '01010', '10010', '11111', '00010', '00010'],
    '5': ['11111', '10000', '11110', '00001', '00001', '10001', '01110'],
    '6': ['00110', '01000', '10000', '11110', '10001', '10001', '01110'],
    '7': ['11111', '00001', '00010', '00100', '01000', '01000', '01000'],
    '8': ['01110', '10001', '10001', '01110', '10001', '10001', '01110'],
    '9': ['01110', '10001', '10001', '01111', '00001', '00010', '01100'],
    ':': ['0', '0', '1', '0', '1', '0', '0']
  };

  var WIDTHS = {
    '0': 5, '1': 5, '2': 5, '3': 5, '4': 5,
    '5': 5, '6': 5, '7': 5, '8': 5, '9': 5, ':': 1
  };

  function pad2(n) {
    return n < 10 ? '0' + n : String(n);
  }

  function layout(text) {
    var glyphs = [];
    var x = 0;
    for (var i = 0; i < text.length; i++) {
      var ch = text.charAt(i);
      var w = WIDTHS[ch] || 5;
      glyphs.push({ ch: ch, x: x, w: w });
      x += w + 1;
    }
    return { glyphs: glyphs, width: Math.max(0, x - 1) };
  }

  var els = null;
  var observer = null;
  var currentText = '';
  var lastAmpm = null;

  function dotPath(ctx, x, y, r) {
    ctx.moveTo(x + r, y);
    ctx.arc(x, y, r, 0, Math.PI * 2);
  }

  function draw() {
    if (!els) return;
    var canvas = els.canvas;
    var ctx = els.ctx;
    var w = canvas.clientWidth;
    var h = canvas.clientHeight;
    if (!w || !h) return;

    var dpr = Math.min(window.devicePixelRatio || 1, 3);
    var bw = Math.round(w * dpr);
    var bh = Math.round(h * dpr);
    if (canvas.width !== bw) canvas.width = bw;
    if (canvas.height !== bh) canvas.height = bh;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    if (!currentText) return;

    var lay = layout(currentText);
    if (!lay.width) return;

    var padX = w * 0.055;
    var padY = h * 0.13;
    var units = Math.max(25, lay.width);
    var pitch = Math.min(
      (w - padX * 2) / units,
      (h - padY * 2) / 7
    );
    var startX = (w - lay.width * pitch) / 2;
    var startY = (h - 7 * pitch) / 2;
    var litR = pitch * 0.36;
    var dimR = pitch * 0.29;

    var lit = [];
    var dim = [];

    for (var g = 0; g < lay.glyphs.length; g++) {
      var glyph = lay.glyphs[g];
      var rows = GLYPHS[glyph.ch];
      if (!rows) continue;
      for (var row = 0; row < 7; row++) {
        var bits = rows[row];
        for (var col = 0; col < glyph.w; col++) {
          var cx = startX + (glyph.x + col + 0.5) * pitch;
          var cy = startY + (row + 0.5) * pitch;
          if (bits.charAt(col) === '1') lit.push([cx, cy]);
          else dim.push([cx, cy]);
        }
      }
    }

    ctx.save();
    ctx.beginPath();
    for (var d = 0; d < dim.length; d++) dotPath(ctx, dim[d][0], dim[d][1], dimR);
    ctx.fillStyle = 'rgba(242, 244, 248, .055)';
    ctx.fill();
    ctx.restore();

    ctx.save();
    ctx.beginPath();
    for (var l = 0; l < lit.length; l++) dotPath(ctx, lit[l][0], lit[l][1], litR);
    ctx.fillStyle = 'rgba(223, 233, 255, .95)';
    ctx.shadowColor = 'rgba(143, 184, 255, .9)';
    ctx.shadowBlur = pitch * 1.05;
    ctx.fill();
    ctx.restore();

    ctx.save();
    ctx.beginPath();
    for (var c2 = 0; c2 < lit.length; c2++) dotPath(ctx, lit[c2][0], lit[c2][1], litR * 0.52);
    ctx.fillStyle = 'rgba(250, 252, 255, .95)';
    ctx.fill();
    ctx.restore();
  }

  function mount(root) {
    root.innerHTML =
      '<div class="clock-card dot-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a"></span>' +
          '<span class="blob blob-b"></span>' +
        '</div>' +
        '<div class="clock-surface dot-surface">' +
          '<div class="dot-panel">' +
            '<canvas class="dot-canvas"></canvas>' +
            '<span class="dot-ampm" hidden></span>' +
          '</div>' +
        '</div>' +
      '</div>';

    var canvas = root.querySelector('.dot-canvas');
    els = {
      canvas: canvas,
      ctx: canvas.getContext('2d'),
      ampm: root.querySelector('.dot-ampm')
    };
    currentText = '';
    lastAmpm = null;

    if (typeof ResizeObserver === 'function') {
      observer = new ResizeObserver(function () { draw(); });
      observer.observe(canvas);
    }
    draw();
  }

  function update(now, api) {
    if (!els) return;
    var state = (api && api.state) || {};
    var hour12 = !!state.hour12;
    var h = now.getHours();
    var text;

    if (hour12) {
      text = (h % 12 || 12) + ':' + pad2(now.getMinutes());
    } else {
      text = pad2(h) + ':' + pad2(now.getMinutes());
    }

    var ampm = hour12 ? (h < 12 ? 'AM' : 'PM') : '';

    if (text !== currentText) {
      currentText = text;
      draw();
    }

    if (ampm !== lastAmpm) {
      lastAmpm = ampm;
      els.ampm.hidden = !hour12;
      els.ampm.textContent = ampm;
    }
  }

  function unmount() {
    if (observer) {
      observer.disconnect();
      observer = null;
    }
    els = null;
    currentText = '';
    lastAmpm = null;
  }

  CLOCKS.dot = {
    id: 'dot',
    name: 'Dot Matrix',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
