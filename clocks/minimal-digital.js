// ClockScreen — Minimal Digital (420x200).
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  function pad2(n) {
    return n < 10 ? '0' + n : String(n);
  }

  function formatDate(now, locale) {
    try {
      return new Intl.DateTimeFormat(locale, {
        weekday: 'long',
        day: 'numeric',
        month: 'long'
      }).format(now);
    } catch (err) {
      try {
        return now.toLocaleDateString(locale);
      } catch (err2) {
        return now.toDateString();
      }
    }
  }

  var els = null;
  var lastTimeKey = '';
  var lastDateKey = '';

  function mount(root) {
    root.innerHTML =
      '<div class="clock-card min-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a"></span>' +
          '<span class="blob blob-b"></span>' +
          '<span class="blob blob-c"></span>' +
        '</div>' +
        '<div class="clock-surface min-surface">' +
          '<div class="min-time">' +
            '<span class="min-main">00:00</span>' +
            '<span class="min-ampm" hidden>AM</span>' +
            '<span class="min-sec">00</span>' +
          '</div>' +
          '<div class="min-date" dir="auto"></div>' +
        '</div>' +
      '</div>';

    els = {
      main: root.querySelector('.min-main'),
      ampm: root.querySelector('.min-ampm'),
      sec: root.querySelector('.min-sec'),
      date: root.querySelector('.min-date')
    };
    lastTimeKey = '';
    lastDateKey = '';
  }

  function update(now, api) {
    if (!els) return;
    var state = (api && api.state) || {};
    var hour12 = !!state.hour12;
    var locale = (api && typeof api.getLocale === 'function') ? api.getLocale() : 'en-US';
    var isArabic = locale.indexOf('ar') === 0;

    var h = now.getHours();
    var m = now.getMinutes();
    var s = now.getSeconds();

    var main;
    var suffix = '';
    if (hour12) {
      var h12 = h % 12 || 12;
      main = pad2(h12) + ':' + pad2(m);
      suffix = h < 12 ? 'AM' : 'PM';
    } else {
      main = pad2(h) + ':' + pad2(m);
    }

    var timeKey = main + '|' + suffix + '|' + (hour12 ? '12' : '24');
    if (timeKey !== lastTimeKey) {
      els.main.textContent = main;
      if (hour12) {
        els.ampm.hidden = false;
        els.ampm.textContent = suffix;
      } else {
        els.ampm.hidden = true;
      }
      lastTimeKey = timeKey;
    }

    var secText = pad2(s);
    if (els.sec.textContent !== secText) els.sec.textContent = secText;

    var dateKey =
      now.getFullYear() + '-' + now.getMonth() + '-' + now.getDate() + '|' + locale;
    if (dateKey !== lastDateKey) {
      els.date.textContent = formatDate(now, locale);
      els.date.classList.toggle('is-arabic', isArabic);
      lastDateKey = dateKey;
    }
  }

  function unmount() {
    els = null;
    lastTimeKey = '';
    lastDateKey = '';
  }

  CLOCKS.minimal = {
    id: 'minimal',
    name: 'Minimal Digital',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
