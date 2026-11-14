# ClockScreen

A frameless, transparent, highly-customizable desktop clock widget for Windows, designed with Apple/macOS aesthetics and Reddit-tier desktop setups in mind.

It supports **8 switchable clock styles**, adaptive low-CPU ticking (< 0.1% idle), ghost mode (click-through), window opacity control, and "Show Desktop" (Win+D) protection.

---

## 8 Built-in Clock Styles

1. **Minimal Digital** (420×200) — Ultra-clean tabular typography, running seconds, weekday and date.
2. **Analog Clean** (360×360) — Minimalist circular dial with smooth 60 FPS sweep second hand.
3. **Flip Clock** (420×210) — Classic split-flap retro digits with realistic 3D fold animations.
4. **Typographic Stack** (320×380) — Modern Swiss / iOS-style stacked numbers (hours over minutes) with date pill.
5. **Glass Card** (400×260) — Translucent glassmorphic card with dynamic time-of-day greeting and full date.
6. **Cyberpunk Neon** (440×210) — Sci-Fi terminal HUD clock with glowing cyan/amber monospace digits and real-time second progress bar.
7. **Word Clock** (520×520) — 11×10 typographic matrix highlighting the time in written words.
8. **Dot Matrix** (420×200) — Retro 5×7 LED canvas matrix display with glowing phosphors.

---

## Requirements

- Node.js 20 or newer
- pnpm (developed against pnpm 11)

---

## Install & Run

```bash
pnpm install
pnpm start
```

---

## Build Installers

```bash
pnpm dist
```

Output goes to `dist/`:
- `ClockScreen-1.0.0-portable.exe` — Single-file portable build.
- `ClockScreen-Setup-1.0.0.exe` — NSIS installer.

The app icon is generated mathematically from code with zero dependencies:
```bash
pnpm icon
```

---

## Controls & Features

- **Drag & Position:** Drag anywhere on the clock to move it. Automatically clamps to your primary or secondary display work area.
- **Mouse Wheel Scaling:** Scroll over the clock to resize smoothly from **50% to 200%**.
- **Context Menu:** Right-click the clock or click the System Tray icon:
  - **Switch Style:** Switch instantly between any of the 8 styles with smooth crossfade.
  - **Window Opacity:** Adjust transparency (100%, 90%, 75%, 60%, 40%) to match any wallpaper.
  - **Always on Top:** Toggle between desktop-level widget or floating above active apps.
  - **Click-Through (Ghost Mode):** When enabled, clicks pass through directly to desktop icons beneath. (Re-enable interaction via the Tray icon).
  - **Lock Position:** Prevent accidental dragging.
  - **12-Hour / 24-Hour:** Toggle AM/PM format.
  - **Date Language:** Choose between System Default locale, English, or Arabic.
  - **Reset Position:** Snaps the widget back to the bottom-right corner.
  - **Start with Windows:** Toggle automatic launch at system login.

---

## Performance & Architecture

- **Smart Adaptive Scheduler:**
  - Clocks that don't need sub-second motion (Digital, Flip, Typographic, Glass, Word, Dot) sleep between seconds and wake up exactly at `1000 - Date.now() % 1000`.
  - Only the Analog sweep second hand uses 60 FPS `requestAnimationFrame`.
  - Result: idle desktop CPU footprint is **< 0.1%**.
- **Smooth Dragging:** Drag calculations run without triggering continuous disk writes. Position persists only when the drag concludes.
- **Win+D Protection:** Windows "Show Desktop" does not cause the clock to disappear; the window restores itself as an omnipresent desktop element.

---

## Settings

Settings are stored as JSON and saved atomically:
```
%APPDATA%\clockscreen\settings.json
```
Includes: `style`, `scale`, `opacity`, `x`, `y`, `locked`, `alwaysOnTop`, `clickThrough`, `hour12`, `locale`, `startWithWindows`. Corrupt or missing files gracefully reset to safe defaults without crashing.
