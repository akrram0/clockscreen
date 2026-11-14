// ClockScreen — Flip Clock (420x210). Split-flap digits, CSS 3D.
'use strict';

(function () {
  var CLOCKS = window.CLOCKS || (window.CLOCKS = {});

  function pad2(n) {
    return n < 10 ? '0' + n : String(n);
  }

  function unitRefs(el) {
    return {
      el: el,
      top: el.querySelector('.flip-top span'),
      bottom: el.querySelector('.flip-bottom span'),
      flapTop: el.querySelector('.flip-flap-top'),
      flapBottom: el.querySelector('.flip-flap-bottom'),
      flapTopText: el.querySelector('.flip-flap-top span'),
      flapBottomText: el.querySelector('.flip-flap-bottom span'),
      value: null
    };
  }

  var els = null;
  var first = true;
  var lastAmpm = null;

  function mount(root, api) {
    root.innerHTML =
      '<div class="clock-card flip-card">' +
        '<div class="blob-layer" aria-hidden="true">' +
          '<span class="blob blob-a"></span>' +
          '<span class="blob blob-b"></span>' +
        '</div>' +
        '<div class="clock-surface flip-surface">' +
          '<div class="flip-row">' +
            '<div class="flip-unit" data-unit="hours">' +
              '<div class="flip-half flip-top"><span>00</span></div>' +
              '<div class="flip-half flip-bottom"><span>00</span></div>' +
              '<div class="flip-half flip-flap-top"><span>00</span></div>' +
              '<div class="flip-half flip-flap-bottom"><span>00</span></div>' +
              '<div class="flip-seam"></div>' +
            '</div>' +
            '<div class="flip-colon" aria-hidden="true"><i></i><i></i></div>' +
            '<div class="flip-unit" data-unit="minutes">' +
              '<div class="flip-half flip-top"><span>00</span></div>' +
              '<div class="flip-half flip-bottom"><span>00</span></div>' +
              '<div class="flip-half flip-flap-top"><span>00</span></div>' +
              '<div class="flip-half flip-flap-bottom"><span>00</span></div>' +
              '<div class="flip-seam"></div>' +
            '</div>' +
            '<span class="flip-ampm" hidden>AM</span>' +
          '</div>' +
        '</div>' +
      '</div>';

    els = {
      hours: unitRefs(root.querySelector('[data-unit="hours"]')),
      minutes: unitRefs(root.querySelector('[data-unit="minutes"]')),
      ampm: root.querySelector('.flip-ampm')
    };
    first = true;
    lastAmpm = null;
    update(new Date(), api);
  }

  function setUnit(unit, value, animate) {
    if (unit.value === value) return;
    var old = unit.value === null ? value : unit.value;

    unit.top.textContent = value;
    unit.bottom.textContent = value;
    unit.flapTopText.textContent = old;
    unit.flapBottomText.textContent = value;
    unit.value = value;

    unit.flapTop.classList.remove('flipping');
    unit.flapBottom.classList.remove('flipping');

    if (!animate) return;

    // Force a reflow so restarting the animation works reliably
    void unit.el.offsetWidth;
    unit.flapTop.classList.add('flipping');
    unit.flapBottom.classList.add('flipping');
  }

  function update(now, api) {
    if (!els) return;
    var state = (api && api.state) || {};
    var hour12 = !!state.hour12;
    var h24 = now.getHours();
    var m = now.getMinutes();

    var hoursText;
    var suffix = '';
    if (hour12) {
      hoursText = pad2(h24 % 12 || 12);
      suffix = h24 < 12 ? 'AM' : 'PM';
    } else {
      hoursText = pad2(h24);
    }

    setUnit(els.hours, hoursText, !first);
    setUnit(els.minutes, pad2(m), !first);

    if (hour12) {
      if (lastAmpm !== suffix) {
        els.ampm.hidden = false;
        els.ampm.textContent = suffix;
        lastAmpm = suffix;
      }
    } else if (lastAmpm !== '') {
      els.ampm.hidden = true;
      lastAmpm = '';
    }

    first = false;
  }

  function unmount() {
    els = null;
    first = true;
    lastAmpm = null;
  }

  CLOCKS.flip = {
    id: 'flip',
    name: 'Flip Clock',
    mount: mount,
    update: update,
    unmount: unmount
  };
})();
