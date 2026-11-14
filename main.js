// ClockScreen — main process.
// Frameless, transparent, desktop-level clock widget with a tray menu.
'use strict';

const { app, BrowserWindow, Menu, Tray, ipcMain, nativeImage, screen } = require('electron');
const fs = require('fs');
const path = require('path');

// ---------- constants ----------
const ICON_PATH = path.join(__dirname, 'build', 'icon.png');
const MARGIN = 32; // gap from the work-area edge for the default position
const SCALE_MIN = 0.5;
const SCALE_MAX = 2.0;

// Base (unscaled) window sizes per style.
const BASE_SIZES = {
  minimal: { width: 420, height: 200 },
  analog: { width: 360, height: 360 },
  flip: { width: 420, height: 210 },
  typographic: { width: 320, height: 380 },
  glass: { width: 400, height: 260 },
  neon: { width: 440, height: 210 },
  word: { width: 520, height: 520 },
  dot: { width: 420, height: 200 },
};

const STYLE_LABELS = [
  ['minimal', 'Minimal Digital'],
  ['analog', 'Analog Clean'],
  ['flip', 'Flip Clock'],
  ['typographic', 'Typographic Stack'],
  ['glass', 'Glass Card'],
  ['neon', 'Cyberpunk Neon'],
  ['word', 'Word Clock'],
  ['dot', 'Dot Matrix'],
];

const DEFAULTS = {
  style: 'minimal',
  scale: 1,
  opacity: 1,
  x: null,
  y: null,
  locked: false,
  alwaysOnTop: false,
  clickThrough: false,
  startWithWindows: false,
  hour12: false,
  locale: 'auto',
  arabicDate: false,
};

// ---------- settings ----------
const settingsPath = () => path.join(app.getPath('userData'), 'settings.json');
let settings = { ...DEFAULTS };

// Settings are user-editable JSON, so never trust them: clamp and coerce on load.
function sanitize(raw) {
  const out = { ...DEFAULTS, ...raw };
  if (!BASE_SIZES[out.style]) out.style = DEFAULTS.style;

  const scale = Number(out.scale);
  out.scale = Number.isFinite(scale)
    ? Math.min(SCALE_MAX, Math.max(SCALE_MIN, Math.round(scale * 10) / 10))
    : DEFAULTS.scale;

  const opacity = Number(out.opacity);
  out.opacity = Number.isFinite(opacity)
    ? Math.min(1, Math.max(0.2, Math.round(opacity * 100) / 100))
    : DEFAULTS.opacity;

  out.x = Number.isFinite(Number(out.x)) ? Number(out.x) : null;
  out.y = Number.isFinite(Number(out.y)) ? Number(out.y) : null;
  for (const key of ['locked', 'alwaysOnTop', 'clickThrough', 'startWithWindows', 'hour12', 'arabicDate']) {
    out[key] = !!out[key];
  }
  if (typeof out.locale !== 'string') out.locale = DEFAULTS.locale;
  return out;
}

function loadSettings() {
  try {
    const raw = JSON.parse(fs.readFileSync(settingsPath(), 'utf8'));
    if (!raw || typeof raw !== 'object') return { ...DEFAULTS };
    return sanitize(raw);
  } catch {
    // Missing or corrupt file: fall back to defaults, never crash.
    return { ...DEFAULTS };
  }
}

let saveTimer = null;

// Atomic write: temp file first, then rename over the real one.
function saveSettingsNow() {
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  try {
    const file = settingsPath();
    const tmp = file + '.tmp';
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(tmp, JSON.stringify(settings, null, 2), 'utf8');
    fs.renameSync(tmp, file);
  } catch (err) {
    console.error('ClockScreen: could not save settings:', err.message);
  }
}

function saveSettings() {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(saveSettingsNow, 300);
}

// ---------- state ----------
function baseSize(style) {
  return BASE_SIZES[style] || BASE_SIZES.minimal;
}

function stateObject() {
  const base = baseSize(settings.style);
  return {
    style: settings.style,
    scale: settings.scale,
    opacity: settings.opacity,
    locked: settings.locked,
    alwaysOnTop: settings.alwaysOnTop,
    clickThrough: settings.clickThrough,
    hour12: settings.hour12,
    locale: settings.locale,
    arabicDate: settings.arabicDate,
    base: { width: base.width, height: base.height },
  };
}

function pushState() {
  if (win && !win.isDestroyed()) win.webContents.send('state', stateObject());
}

// ---------- geometry ----------
function scaledSize(style, scale) {
  const base = baseSize(style);
  return {
    width: Math.round(base.width * scale),
    height: Math.round(base.height * scale),
  };
}

// Keep a rect fully inside the work area of the display it overlaps most.
function clampToDisplays(x, y, width, height) {
  const displays = screen.getAllDisplays();
  let best = displays[0];
  let bestArea = -1;
  for (const d of displays) {
    const wa = d.workArea;
    const ox = Math.max(0, Math.min(x + width, wa.x + wa.width) - Math.max(x, wa.x));
    const oy = Math.max(0, Math.min(y + height, wa.y + wa.height) - Math.max(y, wa.y));
    const area = ox * oy;
    if (area > bestArea) {
      bestArea = area;
      best = d;
    }
  }
  const wa = best.workArea;
  return {
    x: Math.round(Math.min(Math.max(x, wa.x), wa.x + wa.width - width)),
    y: Math.round(Math.min(Math.max(y, wa.y), wa.y + wa.height - height)),
  };
}

function defaultPosition(width, height) {
  const wa = screen.getPrimaryDisplay().workArea;
  return {
    x: wa.x + wa.width - width - MARGIN,
    y: wa.y + wa.height - height - MARGIN,
  };
}

// Resize while keeping the window's centre fixed, then re-clamp on screen.
function resizeKeepingCenter(width, height) {
  if (!win || win.isDestroyed()) return;
  const b = win.getBounds();
  const cx = b.x + b.width / 2;
  const cy = b.y + b.height / 2;
  const pos = clampToDisplays(
    Math.round(cx - width / 2),
    Math.round(cy - height / 2),
    width,
    height
  );
  win.setBounds({ x: pos.x, y: pos.y, width, height });
  settings.x = pos.x;
  settings.y = pos.y;
  saveSettings();
}

// ---------- drag ----------
let dragTimer = null;
let dragOffset = { x: 0, y: 0 };
let isDragging = false;

function stopDrag() {
  if (dragTimer) {
    clearInterval(dragTimer);
    dragTimer = null;
  }
  if (isDragging) {
    isDragging = false;
    saveSettings();
  }
}

function startDrag() {
  if (!win || win.isDestroyed() || settings.locked || settings.clickThrough) return;
  stopDrag();
  isDragging = true;
  const cursor = screen.getCursorScreenPoint();
  const b = win.getBounds();
  dragOffset = { x: cursor.x - b.x, y: cursor.y - b.y };

  dragTimer = setInterval(() => {
    if (!win || win.isDestroyed()) return stopDrag();
    const c = screen.getCursorScreenPoint();
    const bounds = win.getBounds();
    const pos = clampToDisplays(
      Math.round(c.x - dragOffset.x),
      Math.round(c.y - dragOffset.y),
      bounds.width,
      bounds.height
    );
    win.setPosition(pos.x, pos.y, false);
    settings.x = pos.x;
    settings.y = pos.y;
  }, 16);
}

// ---------- window behaviors ----------
function syncAlwaysOnTop() {
  if (!win || win.isDestroyed()) return;
  win.setAlwaysOnTop(!!settings.alwaysOnTop, 'floating');
}

function syncClickThrough() {
  if (!win || win.isDestroyed()) return;
  if (settings.clickThrough) {
    win.setIgnoreMouseEvents(true, { forward: true });
  } else {
    win.setIgnoreMouseEvents(false);
  }
}

function syncOpacity() {
  if (!win || win.isDestroyed()) return;
  const op = Math.min(1, Math.max(0.2, Number(settings.opacity) || 1));
  win.setOpacity(op);
}

function applyStyle(style) {
  if (!BASE_SIZES[style] || style === settings.style) return;
  settings.style = style;
  const size = scaledSize(style, settings.scale);
  resizeKeepingCenter(size.width, size.height);
  saveSettings();
  pushState();
}

function stepScale(dir) {
  const next = Math.round((settings.scale + dir * 0.1) * 10) / 10;
  settings.scale = Math.min(SCALE_MAX, Math.max(SCALE_MIN, next));
  const size = scaledSize(settings.style, settings.scale);
  resizeKeepingCenter(size.width, size.height);
  saveSettings();
  pushState();
}

function syncLoginItem() {
  try {
    app.setLoginItemSettings({
      openAtLogin: !!settings.startWithWindows,
      path: process.execPath,
      args: [],
    });
  } catch (err) {
    console.error('ClockScreen: could not update login item:', err.message);
  }
}

function setSetting(key, value) {
  settings[key] = value;
  saveSettings();
  pushState();
}

// ---------- menu ----------
function buildMenu() {
  const OPACITIES = [
    { label: '100%', value: 1.0 },
    { label: '90%', value: 0.9 },
    { label: '75%', value: 0.75 },
    { label: '60%', value: 0.6 },
    { label: '40%', value: 0.4 },
  ];

  return Menu.buildFromTemplate([
    {
      label: 'Switch style',
      submenu: STYLE_LABELS.map(([id, label]) => ({
        label,
        type: 'radio',
        checked: settings.style === id,
        click: () => {
          applyStyle(id);
          rebuildMenu();
        },
      })),
    },
    {
      label: 'Window Opacity',
      submenu: OPACITIES.map((item) => ({
        label: item.label,
        type: 'radio',
        checked: Math.abs((settings.opacity || 1) - item.value) < 0.06,
        click: () => {
          settings.opacity = item.value;
          syncOpacity();
          saveSettings();
          rebuildMenu();
        },
      })),
    },
    { type: 'separator' },
    {
      label: 'Lock position',
      type: 'checkbox',
      checked: !!settings.locked,
      click: (item) => {
        setSetting('locked', item.checked);
        rebuildMenu();
      },
    },
    {
      label: 'Always on top',
      type: 'checkbox',
      checked: !!settings.alwaysOnTop,
      click: (item) => {
        setSetting('alwaysOnTop', item.checked);
        syncAlwaysOnTop();
        rebuildMenu();
      },
    },
    {
      label: 'Click-through (Ghost mode)',
      type: 'checkbox',
      checked: !!settings.clickThrough,
      click: (item) => {
        setSetting('clickThrough', item.checked);
        syncClickThrough();
        rebuildMenu();
      },
    },
    {
      label: 'Start with Windows',
      type: 'checkbox',
      checked: !!settings.startWithWindows,
      click: (item) => {
        setSetting('startWithWindows', item.checked);
        syncLoginItem();
        rebuildMenu();
      },
    },
    { type: 'separator' },
    {
      label: '12-hour clock',
      type: 'checkbox',
      checked: !!settings.hour12,
      click: (item) => {
        setSetting('hour12', item.checked);
        rebuildMenu();
      },
    },
    {
      label: 'Date Language',
      submenu: [
        {
          label: 'System Default',
          type: 'radio',
          checked: settings.locale === 'auto' && !settings.arabicDate,
          click: () => {
            settings.locale = 'auto';
            settings.arabicDate = false;
            saveSettings();
            pushState();
            rebuildMenu();
          },
        },
        {
          label: 'English',
          type: 'radio',
          checked: settings.locale === 'en' && !settings.arabicDate,
          click: () => {
            settings.locale = 'en';
            settings.arabicDate = false;
            saveSettings();
            pushState();
            rebuildMenu();
          },
        },
        {
          label: 'Arabic',
          type: 'radio',
          checked: settings.arabicDate || settings.locale === 'ar',
          click: () => {
            settings.locale = 'ar';
            settings.arabicDate = true;
            saveSettings();
            pushState();
            rebuildMenu();
          },
        },
      ],
    },
    { type: 'separator' },
    {
      label: 'Reset position to default',
      click: () => {
        const size = scaledSize(settings.style, settings.scale);
        const pos = defaultPosition(size.width, size.height);
        settings.x = pos.x;
        settings.y = pos.y;
        if (win && !win.isDestroyed()) {
          win.setPosition(pos.x, pos.y);
        }
        saveSettings();
      },
    },
    { type: 'separator' },
    {
      label: 'Quit',
      click: () => {
        saveSettingsNow();
        app.quit();
      },
    },
  ]);
}

let menu = null;
let tray = null;

// Rebuild so radio/checkbox checkmarks stay in sync with settings.
function rebuildMenu() {
  menu = buildMenu();
  if (tray && !tray.isDestroyed()) {
    tray.setContextMenu(menu);
    const ghostHint = settings.clickThrough ? ' [Ghost mode: active]' : '';
    tray.setToolTip(`ClockScreen${ghostHint}`);
  }
}

function popupMenu() {
  if (!menu) rebuildMenu();
  if (win && !win.isDestroyed()) menu.popup({ window: win });
  else menu.popup();
}

// ---------- window ----------
let win = null;

function createWindow() {
  const size = scaledSize(settings.style, settings.scale);

  let pos;
  if (Number.isFinite(settings.x) && Number.isFinite(settings.y)) {
    pos = clampToDisplays(settings.x, settings.y, size.width, size.height);
  } else {
    pos = defaultPosition(size.width, size.height);
  }
  settings.x = pos.x;
  settings.y = pos.y;

  win = new BrowserWindow({
    x: pos.x,
    y: pos.y,
    width: size.width,
    height: size.height,
    frame: false,
    transparent: true,
    backgroundColor: '#00000000',
    hasShadow: false,
    resizable: false,
    maximizable: false,
    minimizable: false,
    fullscreenable: false,
    skipTaskbar: true,
    alwaysOnTop: !!settings.alwaysOnTop,
    show: false,
    opacity: settings.opacity || 1.0,
    title: 'ClockScreen',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  win.setSkipTaskbar(true);
  syncAlwaysOnTop();
  syncClickThrough();
  syncOpacity();

  win.loadFile(path.join(__dirname, 'index.html'));

  win.once('ready-to-show', () => {
    win.show();
    pushState();
  });

  // Windows "Show Desktop" (Win+D) protection: restore widget immediately if not always-on-top
  win.on('minimize', (e) => {
    if (!settings.alwaysOnTop) {
      e.preventDefault();
      win.restore();
    }
  });

  // Persist position changes when not actively dragging
  win.on('move', () => {
    if (!win || win.isDestroyed() || isDragging) return;
    const b = win.getBounds();
    settings.x = b.x;
    settings.y = b.y;
    saveSettings();
  });
  win.on('resize', () => saveSettings());

  // A blur means the user clicked elsewhere: end any drag in progress.
  win.on('blur', stopDrag);
  win.on('closed', () => {
    stopDrag();
    win = null;
  });
}

function createTray() {
  let image = nativeImage.createFromPath(ICON_PATH);
  if (!image.isEmpty()) image = image.resize({ width: 16, height: 16 });
  tray = new Tray(image);
  rebuildMenu();
  tray.on('click', () => {
    rebuildMenu();
    tray.popupContextMenu(menu);
  });
}

// ---------- IPC ----------
ipcMain.handle('state:get', () => stateObject());
ipcMain.on('drag:start', startDrag);
ipcMain.on('drag:end', stopDrag);
ipcMain.on('scale:step', (_event, dir) => stepScale(Number(dir) >= 0 ? 1 : -1));
ipcMain.on('menu:open', popupMenu);
ipcMain.on('opacity:set', (_event, val) => {
  settings.opacity = Math.min(1, Math.max(0.2, Number(val) || 1));
  syncOpacity();
  saveSettings();
  rebuildMenu();
});

// ---------- lifecycle ----------
const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (!win || win.isDestroyed()) return;
    if (win.isMinimized()) win.restore();
    win.show();
    win.focus();
  });

  app.whenReady().then(() => {
    settings = loadSettings();
    syncLoginItem();
    createWindow();
    createTray();

    // Re-clamp when displays change (resolution, unplugging a monitor...).
    screen.on('display-metrics-changed', () => {
      if (!win || win.isDestroyed()) return;
      const b = win.getBounds();
      const pos = clampToDisplays(b.x, b.y, b.width, b.height);
      win.setPosition(pos.x, pos.y, false);
    });
  });

  // This app never hides its window, so closing it means quitting.
  app.on('window-all-closed', () => app.quit());
  app.on('before-quit', () => {
    stopDrag();
    saveSettingsNow();
  });
}
