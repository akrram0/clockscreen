// ClockScreen — Glass Card (400x260). Flagship glass surface.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  function pad2(n) {
    return n < 10 ? '0' + n : String(n);
  }

  function formatWeekday(now, locale) {
    try {
      return new Intl.DateTimeFormat(locale, { weekday: 'long' }).format(now);
    } catch (err) {
      try {
        return now.toLocaleDateString(locale, { weekday: 'long' });
      } catch (err2) {
        return now.toDateString().split(' ')[0] || '';
      }
    }
  }

  function formatFullDate(now, locale) {
    try {
      return new Intl.DateTimeFormat(locale, {
        day: 'numeric',
        month: 'long',
        year: 'numeric'
      }).format(now);
    } catch (err) {
      try {
        return now.toLocaleDateString(locale);
      } catch (err2) {
        return now.toDateString();
      }
    }
  }

  function greetingFor(hour) {
    if (hour >= 5 && hour < 12) return 'Good morning';
    if (hour >= 12 && hour < 17) return 'Good afternoon';
    if (hour >= 17 && hour < 22) return 'Good evening';
    return 'Good night';
  }

  var els = null;
  var lastTimeKey = '';
  var lastDayKey = '';
  var lastGreeting = '';

  function mount(root) {
    root.innerHTML =
      '<div class="clock-card glass-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a"></span>' +
          '<span class="blob blob-b"></span>' +
          '<span class="blob blob-c"></span>' +
        '</div>' +
        '<div class="clock-surface glass-surface">' +
          '<div class="glass-top">' +
            '<span class="glass-weekday" dir="auto"></span>' +
            '<span class="glass-greeting"></span>' +
          '</div>' +
          '<div class="glass-time-row">' +
            '<span class="glass-time">00:00</span>' +
            '<span class="glass-ampm" hidden>AM</span>' +
          '</div>' +
          '<div class="glass-date" dir="auto"></div>' +
        '</div>' +
      '</div>';

    els = {
      weekday: root.querySelector('.glass-weekday'),
      greeting: root.querySelector('.glass-greeting'),
      time: root.querySelector('.glass-time'),
      ampm: root.querySelector('.glass-ampm'),
      date: root.querySelector('.glass-date')
    };
    lastTimeKey = '';
    lastDayKey = '';
    lastGreeting = '';
  }

  function update(now, api) {
    if (!els) return;
    var state = (api && api.state) || {};
    var hour12 = !!state.hour12;
    var locale = (api && typeof api.getLocale === 'function') ? api.getLocale() : 'en-US';
    var isArabic = locale.indexOf('ar') === 0;

    var h = now.getHours();
    var m = now.getMinutes();

    var timeText;
    var suffix = '';
    if (hour12) {
      timeText = pad2(h % 12 || 12) + ':' + pad2(m);
      suffix = h < 12 ? 'AM' : 'PM';
    } else {
      timeText = pad2(h) + ':' + pad2(m);
    }

    var timeKey = timeText + '|' + suffix + '|' + (hour12 ? '12' : '24');
    if (timeKey !== lastTimeKey) {
      els.time.textContent = timeText;
      if (hour12) {
        els.ampm.hidden = false;
        els.ampm.textContent = suffix;
      } else {
        els.ampm.hidden = true;
      }
      lastTimeKey = timeKey;
    }

    var greeting = greetingFor(h);
    if (greeting !== lastGreeting) {
      els.greeting.textContent = greeting;
      lastGreeting = greeting;
    }

    var dayKey =
      now.getFullYear() + '-' + now.getMonth() + '-' + now.getDate() + '|' + locale;
    if (dayKey !== lastDayKey) {
      els.weekday.textContent = formatWeekday(now, locale);
      els.date.textContent = formatFullDate(now, locale);
      els.weekday.classList.toggle('is-arabic', isArabic);
      els.date.classList.toggle('is-arabic', isArabic);
      lastDayKey = dayKey;
    }
  }

  function unmount() {
    els = null;
    lastTimeKey = '';
    lastDayKey = '';
    lastGreeting = '';
  }

  CLOCKS.glass = {
    id: 'glass',
    name: 'Glass Card',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
