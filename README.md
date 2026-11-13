# clockscreen

A Windows desktop clock: `HH:MM` drawn straight onto the wallpaper. No card, no panel,
no background, no border — just text. Built in Rust with [Rune](https://github.com/anomalyco/rune).

- 24-hour time, ~220 px tall, bold squared techno face
- translucent near-white fill, so the wallpaper reads through the digits
- one small date line below, wide letter-spacing, 60 % opacity, a hairline rule on each side
- the colon fades once a second (ease-in-out triangle, never a hard cut)
- new digits arrive with a short spring
- transparent, frameless, no taskbar entry, pinned below all other windows
- hold left mouse for 0.5 s to drag; otherwise clicks go to the desktop
- `Ctrl` + wheel scales (0.5×–2.5×)
- tray icon: *Start with Windows*, *Reset position*, *Quit*

Position and scale persist in `%APPDATA%\clockscreen\settings.json`.

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

## How the window works

Rune creates the window (`WindowOptions { transparent, borderless, click_through }`); the rest
is Win32, in `src/win.rs`:

| Want | How |
|---|---|
| Below every other window | `SetWindowPos(hwnd, HWND_BOTTOM, …)` |
| No taskbar button, no Alt-Tab | `WS_EX_TOOLWINDOW`, `WS_EX_APPWINDOW` cleared |
| Clicks reach the desktop | `click_through`, i.e. `WS_EX_TRANSPARENT` |
| Hold-to-drag, `Ctrl`+wheel | a `WH_MOUSE_LL` hook on its own thread, since a click-through window gets no mouse input |
| Start with Windows | `HKCU\…\CurrentVersion\Run` via `reg.exe` |

## Limits, stated plainly

- **No gradient inside the glyphs.** Rune paints a text with one solid colour, and there is no
  rectangular clip (only inset fade masks), so stacking clipped copies to fake a vertical
  gradient is not possible either. The digits are therefore a single translucent near-white
  rather than a true white→grey metallic ramp.
- The build is unsigned, so SmartScreen warns on first run (*More info → Run anyway*).
- CI verifies that it compiles, that the unit tests pass, and that the binary starts. It has no
  GPU, so it cannot verify how the clock actually looks.

## Credits

`assets/ChakraPetch-Bold.ttf` — Chakra Petch, SIL Open Font License (`assets/OFL.txt`).
