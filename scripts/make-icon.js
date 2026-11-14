// Generates build/icon.png — a 256x256 RGBA app icon, drawn with plain math.
// Zero dependencies: fs + zlib only.
'use strict';

const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const SIZE = 256;
const CENTER = SIZE / 2;

// ---------- tiny PNG encoder (IHDR / IDAT / IEND) ----------
const crcTable = (() => {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([len, body, crc]);
}

function encodePNG(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  ihdr[10] = 0; // deflate
  ihdr[11] = 0; // adaptive filtering
  ihdr[12] = 0; // no interlace

  // One filter byte (0 = None) per scanline, then raw RGBA pixels.
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0;
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

// ---------- drawing helpers ----------
const pixels = Buffer.alloc(SIZE * SIZE * 4); // transparent canvas

// Source-over blend of a flat color at coverage `a` (0..1).
function blend(x, y, r, g, b, a) {
  if (a <= 0) return;
  const i = (y * SIZE + x) * 4;
  const dstA = pixels[i + 3] / 255;
  const outA = a + dstA * (1 - a);
  if (outA <= 0) return;
  pixels[i] = Math.round((r * a + pixels[i] * dstA * (1 - a)) / outA);
  pixels[i + 1] = Math.round((g * a + pixels[i + 1] * dstA * (1 - a)) / outA);
  pixels[i + 2] = Math.round((b * a + pixels[i + 2] * dstA * (1 - a)) / outA);
  pixels[i + 3] = Math.round(outA * 255);
}

// Signed distance to a rounded rectangle centred at `cx,cy` (negative = inside).
function sdRoundRect(x, y, cx, cy, halfW, halfH, r) {
  const dx = Math.abs(x - cx) - (halfW - r);
  const dy = Math.abs(y - cy) - (halfH - r);
  const ox = Math.max(dx, 0);
  const oy = Math.max(dy, 0);
  return Math.min(Math.max(dx, dy), 0) + Math.hypot(ox, oy) - r;
}

// Distance from point to a line segment (used for the clock hands).
function sdSegment(x, y, ax, ay, bx, by) {
  const px = x - ax;
  const py = y - ay;
  const vx = bx - ax;
  const vy = by - ay;
  const t = Math.max(0, Math.min(1, (px * vx + py * vy) / (vx * vx + vy * vy)));
  return Math.hypot(px - vx * t, py - vy * t);
}

// Coverage from a signed distance: 1px anti-aliased edge.
const cover = (d) => Math.max(0, Math.min(1, 0.5 - d));

// ---------- the icon ----------
const CARD_HALF = 112; // 224x224 card, 16px transparent margin
const CARD_R = 56;
const RING_R = 72;
const STROKE = 6;

const CARD = [0x11, 0x13, 0x18]; // #111318
const BORDER = [0x2a, 0x2f, 0x3a]; // #2a2f3a
const FG = [0xe8, 0xea, 0xf0]; // #e8eaf0

// Hands: 10 o'clock (hour) and 2 o'clock (minute).
const hourAngle = (10 / 12) * Math.PI * 2; // clockwise from 12
const minAngle = (10 / 60) * Math.PI * 2;
const hourTip = [CENTER + Math.sin(hourAngle) * 40, CENTER - Math.cos(hourAngle) * 40];
const minTip = [CENTER + Math.sin(minAngle) * 58, CENTER - Math.cos(minAngle) * 58];

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const px = x + 0.5;
    const py = y + 0.5;

    // Card body.
    const dCard = sdRoundRect(px, py, CENTER, CENTER, CARD_HALF, CARD_HALF, CARD_R);
    blend(x, y, ...CARD, cover(dCard));

    // 1px inset border band (just inside the card edge).
    const dBorder = sdRoundRect(px, py, CENTER, CENTER, CARD_HALF - 1, CARD_HALF - 1, CARD_R - 1);
    const band = Math.min(cover(dBorder), cover(-(dBorder + 1)));
    blend(x, y, ...BORDER, band);

    // Clock ring.
    const dRing = Math.abs(Math.hypot(px - CENTER, py - CENTER) - RING_R) - STROKE / 2;
    blend(x, y, ...FG, cover(dRing));

    // Hands (rounded caps come free from the capsule distance).
    const dHour = sdSegment(px, py, CENTER, CENTER, hourTip[0], hourTip[1]) - STROKE / 2;
    const dMin = sdSegment(px, py, CENTER, CENTER, minTip[0], minTip[1]) - STROKE / 2;
    blend(x, y, ...FG, cover(Math.min(dHour, dMin)));

    // Center dot.
    const dDot = Math.hypot(px - CENTER, py - CENTER) - 5;
    blend(x, y, ...FG, cover(dDot));
  }
}

const outPath = path.join(__dirname, '..', 'build', 'icon.png');
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, encodePNG(SIZE, SIZE, pixels));
console.log('icon written:', outPath);
