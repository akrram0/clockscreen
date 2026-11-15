# ClockScreen

A high-performance, native Windows desktop **Wallpaper Depth-Effect Clock** and **ClockScreen Studio Management App**.

Tucks giant stylized clock digits (`04:17`) directly behind wallpaper subjects (such as character hair, fish fins, and architectural edges) with metallic chrome gradients and specular rim lighting, accompanied by custom faces, tints, and desktop shell integration.

---

## Features

- **True 3D Wallpaper Depth Occlusion**:
  Sandwiches clock numbers behind foreground subjects on your wallpaper with genuine 3D depth.
- **Pure Win32 `UpdateLayeredWindow` Engine**:
  Zero white boxes, zero swapchain fallback artifacts, and 100% mathematically flawless alpha compositing via 32-bit pre-multiplied DIBs.
- **Blazing Performance (0% Idle CPU / < 8 MB RAM)**:
  Instantaneous cold startup (< 15ms), zero GPU spinup, and 0% idle CPU. Wakes only at the top of the minute.
- **Desktop Shell Pinning (`HWND_BOTTOM`)**:
  Permanently anchored to the Windows desktop canvas. Intercepts `WM_WINDOWPOSCHANGING` so the clock never vanishes or minimizes when you press **Win + D** (Show Desktop).
- **Smooth Native Dragging & Scaling**:
  When unlocked, drag the clock smoothly with native `HTCAPTION` tracking at your monitor's full refresh rate (120Hz/144Hz/240Hz) with zero cursor stutter. Hold `Ctrl` and scroll mouse wheel to scale smoothly.
- **Click-Through Mode**:
  Toggleable click-through (`WS_EX_TRANSPARENT`) so you can click desktop icons straight through the clock numbers.
- **ClockScreen Studio Management App**:
  A dedicated control center built following Apple/macOS Sonoma dark-glass design tokens to visually customize clock faces, appearance tints, size, opacity, 12h/24h format, and depth occlusion.
- **System Tray Integration**:
  Quick right-click context menu in the Windows notification area to toggle locking, cycle faces and tints, reset position, or toggle Start with Windows.

---

## Typography & Clock Faces

All fonts are bundled and SIL Open Font License (OFL) licensed:
* **Glass**: Frosted chrome ramp with vertical sheen and specular rim light (Hero Face).
* **Hairline**: Clean outer stroke outlines with wide tracking.
* **Bold**: Heavy squared techno numbers with high contrast.
* **Stacked**: Hours stacked over minutes.
* **Condensed**: Narrow DIN-style block.
* **Mono**: Monospaced developer typography.

---

## Building

```powershell
cargo build --release
```

Output executable sits at `target/release/clockscreen.exe`.

---

## Configuration

Settings are stored in `%APPDATA%\clockscreen\settings.json` and synchronize in real time.

---

## License

MIT License. Bundled fonts are licensed under SIL OFL.
