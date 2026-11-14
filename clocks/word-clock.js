// ClockScreen — Word Clock (520x520). 11x10 letter grid.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  var ROWS = [
    'ITLISASAMPM',
    'ACQUARTERDC',
    'TWENTYFIVEX',
    'HALFSTENFTO',
    'PASTERUNINE',
    'ONESIXTHREE',
    'FOURFIVETWO',
    'EIGHTELEVEN',
    'SEVENTWELVE',
    'TENSEOCLOCK'
  ];

  // Word cells, keyed by role. Ranges are inclusive (row : colStart-colEnd).
  var W = {
    it:       { row: 0, from: 0, to: 1 },
    is:       { row: 0, from: 3, to: 4 },
    am:       { row: 0, from: 7, to: 8 },
    pm:       { row: 0, from: 9, to: 10 },
    quarter:  { row: 1, from: 2, to: 8 },
    twenty:   { row: 2, from: 0, to: 5 },
    fiveMin:  { row: 2, from: 6, to: 9 },
    half:     { row: 3, from: 0, to: 3 },
    tenMin:   { row: 3, from: 5, to: 7 },
    to:       { row: 3, from: 9, to: 10 },
    past:     { row: 4, from: 0, to: 3 },
    nine:     { row: 4, from: 7, to: 10 },
    one:      { row: 5, from: 0, to: 2 },
    six:      { row: 5, from: 3, to: 5 },
    three:    { row: 5, from: 6, to: 10 },
    four:     { row: 6, from: 0, to: 3 },
    fiveHour: { row: 6, from: 4, to: 7 },
    two:      { row: 6, from: 8, to: 10 },
    eight:    { row: 7, from: 0, to: 4 },
    eleven:   { row: 7, from: 5, to: 10 },
    seven:    { row: 8, from: 0, to: 4 },
    twelve:   { row: 8, from: 5, to: 10 },
    tenHour:  { row: 9, from: 0, to: 2 },
    oclock:   { row: 9, from: 5, to: 10 }
  };

  var HOUR_WORDS = [
    null, W.one, W.two, W.three, W.four, W.fiveHour, W.six,
    W.seven, W.eight, W.nine, W.tenHour, W.eleven, W.twelve
  ];

  var els = null;
  var lastKey = '';

  function wordsFor(now, hour12) {
    var minute = Math.floor(now.getMinutes() / 5) * 5;
    var hour24 = now.getHours();
    var hour = hour24;
    var out = [W.it, W.is];

    if (minute >= 35) hour = (hour + 1) % 24;

    var h12 = hour % 12 || 12;
    out.push(HOUR_WORDS[h12]);

    switch (minute) {
      case 0:
        out.push(W.oclock);
        break;
      case 5:
        out.push(W.fiveMin, W.past);
        break;
      case 10:
        out.push(W.tenMin, W.past);
        break;
      case 15:
        out.push(W.quarter, W.past);
        break;
      case 20:
        out.push(W.twenty, W.past);
        break;
      case 25:
        out.push(W.twenty, W.fiveMin, W.past);
        break;
      case 30:
        out.push(W.half, W.past);
        break;
      case 35:
        out.push(W.twenty, W.fiveMin, W.to);
        break;
      case 40:
        out.push(W.twenty, W.to);
        break;
      case 45:
        out.push(W.quarter, W.to);
        break;
      case 50:
        out.push(W.tenMin, W.to);
        break;
      case 55:
        out.push(W.fiveMin, W.to);
        break;
      default:
        break;
    }

    if (hour12) out.push(hour24 < 12 ? W.am : W.pm);
    return out;
  }

  function mount(root) {
    root.innerHTML =
      '<div class="clock-card word-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a"></span>' +
          '<span class="blob blob-b"></span>' +
          '<span class="blob blob-c"></span>' +
        '</div>' +
        '<div class="clock-surface word-surface">' +
          '<div class="word-grid" role="img" aria-label="Word clock"></div>' +
        '</div>' +
      '</div>';

    var grid = root.querySelector('.word-grid');
    var cells = [];
    var frag = document.createDocumentFragment();

    for (var r = 0; r < ROWS.length; r++) {
      for (var c = 0; c < ROWS[r].length; c++) {
        var cell = document.createElement('span');
        cell.className = 'word-cell';
        cell.textContent = ROWS[r].charAt(c);
        cells.push(cell);
        frag.appendChild(cell);
      }
    }

    grid.appendChild(frag);
    els = { cells: cells, grid: grid };
    lastKey = '';
  }

  function update(now, api) {
    if (!els) return;
    var state = (api && api.state) || {};
    var hour12 = !!state.hour12;
    var bucket = Math.floor(now.getMinutes() / 5);
    var key = now.getHours() + ':' + bucket + ':' + (hour12 ? '12' : '24');
    if (key === lastKey) return;
    lastKey = key;

    var words = wordsFor(now, hour12);
    var active = new Set();
    for (var i = 0; i < words.length; i++) {
      var w = words[i];
      for (var c = w.from; c <= w.to; c++) active.add(w.row * 11 + c);
    }

    for (var idx = 0; idx < els.cells.length; idx++) {
      var on = active.has(idx);
      var cell = els.cells[idx];
      if (cell.classList.contains('active') !== on) {
        cell.classList.toggle('active', on);
      }
    }
  }

  function unmount() {
    els = null;
    lastKey = '';
  }

  CLOCKS.word = {
    id: 'word',
    name: 'Word Clock',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
