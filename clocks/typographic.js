// ClockScreen — Typographic Stack (320x380).
// Modern Swiss / Apple lockscreen stacked typographic clock.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  function pad2(n) {
    return n < 10 ? '0' + n : String(n);
  }

  function formatDate(now, locale) {
    try {
      return new Intl.DateTimeFormat(locale, {
        weekday: 'short',
        month: 'short',
        day: 'numeric'
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
      '<div class="clock-card typo-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a typo-blob-a"></span>' +
          '<span class="blob blob-b typo-blob-b"></span>' +
        '</div>' +
        '<div class="clock-surface typo-surface">' +
          '<div class="typo-stack">' +
            '<div class="typo-hours">00</div>' +
            '<div class="typo-minutes">00</div>' +
          '</div>' +
          '<div class="typo-footer">' +
            '<span class="typo-date" dir="auto">SAT, NOV 14</span>' +
            '<span class="typo-ampm" hidden>AM</span>' +
          '</div>' +
        '</div>' +
      '</div>';

    els = {
      hours: root.querySelector('.typo-hours'),
      minutes: root.querySelector('.typo-minutes'),
      date: root.querySelector('.typo-date'),
      ampm: root.querySelector('.typo-ampm')
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

    var hoursStr;
    var suffix = '';
    if (hour12) {
      hoursStr = pad2(h % 12 || 12);
      suffix = h < 12 ? 'AM' : 'PM';
    } else {
      hoursStr = pad2(h);
    }
    var minStr = pad2(m);

    var timeKey = hoursStr + '|' + minStr + '|' + suffix;
    if (timeKey !== lastTimeKey) {
      els.hours.textContent = hoursStr;
      els.minutes.textContent = minStr;
      if (hour12) {
        els.ampm.hidden = false;
        els.ampm.textContent = suffix;
      } else {
        els.ampm.hidden = true;
      }
      lastTimeKey = timeKey;
    }

    var dateKey =
      now.getFullYear() + '-' + now.getMonth() + '-' + now.getDate() + '|' + locale;
    if (dateKey !== lastDateKey) {
      els.date.textContent = formatDate(now, locale).toUpperCase();
      els.date.classList.toggle('is-arabic', isArabic);
      lastDateKey = dateKey;
    }
  }

  function unmount() {
    els = null;
    lastTimeKey = '';
    lastDateKey = '';
  }

  CLOCKS.typographic = {
    id: 'typographic',
    name: 'Typographic Stack',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
