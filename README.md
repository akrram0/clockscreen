# ClockScreen

A native, GPU-accelerated desktop **Wallpaper Depth Effect Clock** and **Desktop Management App** for Windows, powered by **[anomalyco/rune](https://github.com/anomalyco/rune)**.

Tucks giant stylized clock numbers behind wallpaper subjects (such as character hair, fish fins, and architectural edges) with metallic chrome gradients and rim lighting, accompanied by a dynamic media island and circular music vinyl visualizer.

---

## Features

- **Wallpaper Depth Occlusion**:
  Renders stylized display typography nestled directly *behind* foreground subjects on your wallpaper with genuine 3D depth.
- **Pure Native GPU Rendering**:
  Built on **`anomalyco/rune`** with `wgpu` (DirectX 12 / Vulkan), 120 FPS spring animations, and zero Chromium or Electron overhead (< 25 MB RAM, 0% idle CPU).
- **True Desktop Shell Integration**:
  Pins directly to the Windows `WorkerW` / `Progman` desktop layer via Win32 API (`pin_to_desktop`), sitting beneath desktop icons while maintaining transparent mouse click-through.
- **Hold-to-Drag & Ctrl+Wheel Scaling**:
  Even though the window is click-through, a low-level mouse hook allows you to click-and-hold to drag the clock across monitors or press `Ctrl + Scroll` to resize smoothly.
- **Companion Widgets**:
  - **Top Dynamic Island**: Minimal pill displaying current track title, artist, and playback activity.
  - **Circular Music Vinyl Visualizer**: Stylized spinning vinyl disc displaying live lyrics and audio visualizer needles.
  - **Tracked Date Line**: Clean typography with horizontal hairlines.
- **Desktop Management App (Studio Settings)**:
  A dedicated control center built with Rune's Apple/macOS dark-glass design language to configure clock faces (Glass, Hairline, Bold, Stacked, Condensed, Mono), tinting (Auto, Light, Warm, Cool), opacity, scale, and companion widget toggles.

---

## Requirements

- Windows 10 or 11 (x64)
- Rust 1.80+ (Stable)
- PowerShell (for checking out Rune vendor subtrees)

---

## Building Locally

```powershell
# 1. Fetch pinned Rune vendor crates
./scripts/fetch-rune.ps1

# 2. Build the release binary
cargo build --release

# 3. Output binary sits in target/release/clockscreen.exe
```

---

## Configuration

Settings are stored as JSON and reloaded in real time:
```
%APPDATA%\clockscreen\settings.json
```

---

## License

MIT License. Bundled fonts are licensed under the SIL Open Font License (OFL).
