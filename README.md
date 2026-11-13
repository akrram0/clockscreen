# clockscreen

A Windows desktop clock: `HH:MM` drawn straight onto the wallpaper. No card, no panel,
no background, no border — just text. Built in Rust with [Rune](https://github.com/anomalyco/rune).

- `HH:MM` at ~220 pt, in one of six faces, with a real vertical glass ramp and a rim light
- every digit gets the same advance width, so the time never shifts as the minutes change
- one small date line below, wide letter-spacing, 60 % opacity, a hairline rule on each side
- the colon breathes once a second (ease-in-out, never a hard cut)
- new digits arrive with a short spring
- transparent, frameless, no taskbar entry, pinned below all other windows
- hold left mouse for 0.5 s to drag; otherwise clicks go to the desktop
- `Ctrl` + wheel scales (0.5×–2.5×)
- tray icon: *Settings…*, *Next face*, *Start with Windows*, *Reset position*, *Quit*

Position, size, face, tint, opacity and 12/24-hour format persist in
`%APPDATA%\clockscreen\settings.json`. Settings written by 1.0.0 still load.

## Download

[Releases](https://github.com/akrram0/clockscreen/releases/latest) carry a prebuilt
`clockscreen.exe` (Windows x64, no installer, no runtime). It is unsigned, so SmartScreen warns
on first run: *More info → Run anyway*.

## Run

```powershell
.\scripts\fetch-rune.ps1   # once: Rune's patched crates must exist under .rune/
cargo run --release
```

The first build compiles Rune and wgpu from git, so it takes a while.

## Build

```powershell
.\scripts\fetch-rune.ps1
cargo build --release
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target\release\clockscreen.exe dist\clockscreen.exe
```

The exe lands at `dist\clockscreen.exe` — a single self-contained binary, no installer,
no runtime dependencies.

CI (`.github/workflows/build.yml`) does the same on `windows-latest`, runs the unit tests,
smoke-runs the binary, and uploads `clockscreen.exe` as a build artifact.

## Faces

| Face | What it is |
|---|---|
| Glass | Frosted: the vertical white→grey ramp plus a bright rim on the up-facing edges |
| Hairline | Only the rim of each stroke survives, so the digits read as thin outlines |
| Bold | Heavier and tighter than Glass, with a harder fill |
| Stacked | Hours above minutes, for tall and narrow wallpaper areas |
| Condensed | Narrower face pulled together into a block |
| Mono | Monospaced, for a technical look |

Tint: *Auto* follows the system theme (dark theme → light glass, light theme → graphite glass),
and *Light* / *Warm* / *Cool* pin it.

## How the time is drawn

Rune paints text with one solid colour and has no rectangular clip, so a gradient inside the
glyphs is impossible with `text()`. The time and the date are therefore rasterised in-process
with [`fontdue`](https://github.com/mooman219/fontdue) into an RGBA buffer — vertical ramp, rim
light, fixed-width digit cells — and handed to Rune as an image (`Pixels` + `El::pixels`).

The window is **measured, not guessed**: the layout is rasterised once before the window exists,
and the measured size (plus padding) is passed to `WindowOptions::physical_size`. Rune clips
overflow rather than growing the window, so a fixed size is what cut the clock off on the right.

## How the window works

Rune creates the windows; the rest is Win32, in `src/win.rs`:

| Want | How |
|---|---|
| Below every other window | `SetWindowPos(hwnd, HWND_BOTTOM, …)` |
| No taskbar button, no Alt-Tab | `WS_EX_TOOLWINDOW`, `WS_EX_APPWINDOW` cleared |
| Clicks reach the desktop | `click_through`, i.e. `WS_EX_TRANSPARENT` |
| Hold-to-drag, `Ctrl`+wheel | a `WH_MOUSE_LL` hook on its own thread, since a click-through window gets no mouse input |
| Start with Windows | `HKCU\…\CurrentVersion\Run` via `reg.exe` |

The settings window is an ordinary decorated Rune window, created hidden at startup and shown
from the tray. It owns no shared state: every change is written to `settings.json`, and the
clock re-reads that file on its next poll.

## Limits, stated plainly

- The build is unsigned, so SmartScreen warns on first run (*More info → Run anyway*).
- CI verifies that it compiles, that the unit tests pass, and that the binary starts. It has no
  GPU, so it cannot verify how the clock actually looks.
- *Auto* tint follows the system theme, not the wallpaper under the clock: Rune exposes no
  screen sampling.

## Credits

All three faces are SIL Open Font License, and all three have identical advance widths for
`0`–`9` (verified, not assumed):

- `assets/Oxanium-Bold.ttf` — Oxanium (`assets/OFL-Oxanium.txt`)
- `assets/TitilliumWeb-Bold.ttf` — Titillium Web (`assets/OFL-TitilliumWeb.txt`)
- `assets/JetBrainsMono-Bold.ttf` — JetBrains Mono (`assets/OFL-JetBrainsMono.txt`)
