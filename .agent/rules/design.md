# ClockScreen Design System & Token Rules

## Aesthetic Direction
- **Identity:** Apple/macOS Sonoma & Ventura desktop widget aesthetic — restrained, dark-first, functional glassmorphism, generous whitespace, crisp legibility against variable desktop wallpapers.
- **Typography:** System font stack (`-apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Inter", "Segoe UI", system-ui, sans-serif`), strict 2-weight limit per screen (Display: Light 200/300, Accents: Regular 400 / Medium 500).

---

## Token Scale

### 1. Color Palette (Exact Hex & Precise Alpha)
- Surface Deep (Base Canvas): `#0a0c10`
- Glass Surface Tint: `rgba(18, 21, 29, 0.76)`
- Glass Surface High: `rgba(26, 30, 42, 0.88)`
- Hairline Border: `rgba(255, 255, 255, 0.12)`
- Subdued Border: `rgba(255, 255, 255, 0.06)`
- Foreground Primary (Ink): `#f4f6fb`
- Foreground Secondary (Muted): `#9ba3b8`
- Foreground Tertiary (Faint): `#434b5c`
- Accent Primary (Electric Blue): `#38bdf8`
- Accent Warm (Amber / Braun): `#f59e0b`
- Highlight Shimmer: `rgba(255, 255, 255, 0.14)`

### 2. Spacing Scale (8pt Grid Based)
- `4px` (0.25rem) — micro gaps, colon spacing, inner chip padding
- `8px` (0.5rem) — element spacing, badge margins
- `16px` (1.0rem) — card inner padding (compact), layout gaps
- `24px` (1.5rem) — card standard padding, section separators
- `32px` (2.0rem) — large card padding, outer breathing room
- `48px` (3.0rem) — major container padding
- `64px` (4.0rem) — maximum layout bounds

### 3. Border Radius Tiers
- **sm (`6px`):** Inputs, tags, indicators, split-flap seam notches
- **md (`14px`):** Flip digit units, secondary cards, nested panels
- **lg (`24px`):** Primary outer widget container, main clock cards
- **full (`9999px`):** Round analog dials, circular indicators, pills, avatars

### 4. Glassmorphism & Elevation
- Backdrop filter: `blur(20px) saturate(150%)`
- Card background: `rgba(18, 21, 29, 0.78)`
- Specular edge: `inset 0 1px 0 rgba(255, 255, 255, 0.12), inset 0 0 0 1px rgba(255, 255, 255, 0.03)`
- Drop shadow: `0 16px 36px rgba(0, 0, 0, 0.45), 0 4px 12px rgba(0, 0, 0, 0.25)`

### 5. Typography Scale
- **Display Extra-Large:** `6.5rem` / weight `200` / tracking `-0.03em` (Minimal Digital primary time)
- **Display Large:** `4.2rem` / weight `300` / tracking `-0.02em` (Flip digits, Glass time)
- **Subhead / Date:** `0.875rem` (14px) / weight `400` / tracking `0.02em`
- **Caption / Status:** `0.75rem` (12px) / weight `500` / tracking `0.12em` uppercase (AM/PM, seconds, tags)
