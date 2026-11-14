//! ClockScreen: Native GPU Wallpaper Depth Clock & Manager in Rune.
//!
//! The time is rasterised in [`glyphs`] with vertical glass/chrome ramps and specular rim lights.
//! Rune owns the window; Win32 puts it at the bottom of the z-order, keeps it out of the taskbar,
//! and watches the mouse so the clock can be dragged and scaled while remaining click-through.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod depth;
mod faces;
mod glyphs;
mod media;
mod panel;
mod settings;
mod win;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use rune::{
    col, el, row, text, AppSpec, Color, Cx, El, Family, LoopSpec, Spring, TextStyle, Tx,
    WindowHandle, WindowOptions, WindowSpec,
};

/// Breathing room around the content, in points.
const PAD: f32 = 44.0;

static INPUT: OnceLock<Arc<win::Input>> = OnceLock::new();
static WINDOW: Mutex<Option<WindowHandle>> = Mutex::new(None);
static DPI: OnceLock<f32> = OnceLock::new();
static SYSTEM_DARK: AtomicBool = AtomicBool::new(true);

/// The clock re-reads `settings.json` on its next poll.
pub(crate) fn request_clock_reload() {
    if let Some(input) = INPUT.get() {
        input.request_reload();
    }
}

/// The clock moves itself back to the centre of the primary display.
pub(crate) fn request_clock_reset() {
    if let Some(input) = INPUT.get() {
        input.request_reset();
    }
}

/// The settings panel re-reads `settings.json` on its next poll.
pub(crate) fn request_panel_reload() {
    if let Some(input) = INPUT.get() {
        input.request_panel();
    }
}

pub(crate) fn take_panel_reload() -> bool {
    INPUT.get().map(|input| input.take_panel()).unwrap_or(false)
}

const WEEKDAYS: [&str; 7] =
    ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

/// One rendered time: four runs, so the colon can blink on its own without breaking the ramp.
struct Time {
    hh: glyphs::Run,
    colon: glyphs::Run,
    mm: glyphs::Run,
    date: glyphs::Run,
    stacked: bool,
    gap: f32,
    date_gap: f32,
    hairline: Color,
}

struct State {
    hh: String,
    mm: String,
    date: String,
    minute: u32,
    scale: f32,
    fade: f32,
    fade_pending: bool,
    face: settings::Face,
    tint: settings::Tint,
    hour24: bool,
    opacity: f32,
    depth_enabled: bool,
    show_date: bool,
    show_vinyl: bool,
    show_island: bool,
    system_dark: bool,
    time: Option<Time>,
    /// Window size in points, padding included. Measured, never guessed: Rune clips overflow.
    window: (f32, f32),
}

fn two(n: u32) -> String {
    format!("{n:02}")
}

fn date_line(now: &win::Now, meridiem: &str, hour24: bool) -> String {
    let weekday = WEEKDAYS.get(now.weekday as usize).copied().unwrap_or("");
    let month = MONTHS.get(now.month.saturating_sub(1) as usize).copied().unwrap_or("");
    if hour24 {
        format!("{weekday} {} {month}", now.day)
    } else {
        format!("{weekday} {} {month}  ·  {meridiem}", now.day)
    }
}

fn clock_parts(now: &win::Now, hour24: bool) -> (String, String, &'static str) {
    if hour24 {
        (two(now.hour), two(now.minute), "")
    } else {
        let hour = now.hour % 12;
        let hour = if hour == 0 { 12 } else { hour };
        let meridiem = if now.hour < 12 { "AM" } else { "PM" };
        (two(hour), two(now.minute), meridiem)
    }
}

fn colour(rgb: [u8; 3], alpha: f32) -> Color {
    Color {
        r: rgb[0] as f32 / 255.0,
        g: rgb[1] as f32 / 255.0,
        b: rgb[2] as f32 / 255.0,
        a: alpha,
    }
}

fn dpi() -> f32 {
    DPI.get().copied().unwrap_or(1.0)
}

/// Rasterise the current time and work out how big the window has to be for it.
fn build(s: &mut State) {
    let style = faces::style(s.face, s.tint, s.system_dark, s.opacity);
    let Some(font) = glyphs::font(style.font) else { return };
    let spec = faces::spec(&style, dpi(), style.size_pt * s.scale);
    let (Some(hh), Some(colon), Some(mm)) = (
        glyphs::run(font, &s.hh, &spec),
        glyphs::run(font, ":", &spec),
        glyphs::run(font, &s.mm, &spec),
    ) else {
        return;
    };

    let gap = 2.0 * s.scale;
    let (width, height) = if style.stacked {
        (
            (hh.points.0 + colon.points.0).max(mm.points.0),
            hh.points.1 + gap + mm.points.1,
        )
    } else {
        (
            hh.points.0 + colon.points.0 + mm.points.0 + gap * 2.0,
            hh.points.1.max(mm.points.1),
        )
    };

    let Some(date) = glyphs::run(font, &s.date, &faces::date_spec(&style, dpi(), s.scale)) else {
        return;
    };
    let date_gap = style.date_gap * s.scale;

    let vinyl_w = if s.show_vinyl { 180.0 * s.scale + 36.0 * s.scale } else { 0.0 };
    let island_h = if s.show_island { 48.0 * s.scale } else { 0.0 };
    let date_row = if s.show_date { date.points.0 + 2.0 * 96.0 * s.scale + 2.0 * 16.0 * s.scale } else { 0.0 };

    let total_w = (width + vinyl_w).max(date_row) + PAD * 2.0;
    let total_h = height + date_gap + date.points.1 + island_h + PAD * 2.0;
    s.window = (total_w, total_h);

    s.time = Some(Time {
        hh,
        colon,
        mm,
        date,
        stacked: style.stacked,
        gap,
        date_gap,
        hairline: colour(style.ink.top, 0.20),
    });
}

fn init() -> State {
    let saved = settings::load();
    let now = win::now();
    let hour24 = saved.hour24;
    let (hh, mm, meridiem) = clock_parts(&now, hour24);
    let mut state = State {
        date: date_line(&now, meridiem, hour24),
        hh,
        mm,
        minute: now.hour * 60 + now.minute,
        scale: INPUT.get().map(|input| input.scale()).unwrap_or(saved.scale),
        fade: 1.0,
        fade_pending: false,
        face: saved.face,
        tint: saved.tint,
        hour24,
        opacity: saved.opacity,
        depth_enabled: saved.depth_enabled,
        show_date: saved.show_date,
        show_vinyl: saved.show_vinyl,
        show_island: saved.show_island,
        system_dark: SYSTEM_DARK.load(Ordering::Relaxed),
        time: None,
        window: (0.0, 0.0),
    };
    build(&mut state);
    state
}

fn view(s: &State, cx: &mut Cx<State>) -> El {
    SYSTEM_DARK.store(cx.system_dark(), Ordering::Relaxed);

    let k = s.scale;
    let settle = || Spring::new(0.22, 0.45);
    let track = media::get_current_track();
    let accent = Color { r: 0.22, g: 0.74, b: 0.97, a: 0.90 }; // #38bdf8

    let mut root = col().w_full().h_full().center().gap(16.0 * k);

    // 1. Top Dynamic Island (if enabled)
    if s.show_island {
        root = root.child(depth::dynamic_island(k, &track.title, &track.artist, accent));
    }

    match &s.time {
        Some(time) => {
            let colon = el()
                .size(time.colon.points.0, time.colon.points.1)
                .pixels(&time.colon.pixels)
                .looping(LoopSpec::opacity(1.0, 0.0, 0.18, 1.0));

            let digits = |run: &glyphs::Run| {
                el()
                    .size(run.points.0, run.points.1)
                    .pixels(&run.pixels)
                    .opacity(s.fade)
                    .opacity_motion(settle())
            };

            let time_block = if time.stacked {
                col()
                    .center()
                    .gap(time.gap)
                    .child(row().center_y().child(digits(&time.hh)).child(colon))
                    .child(digits(&time.mm))
            } else {
                row()
                    .center_y()
                    .gap(time.gap)
                    .child(digits(&time.hh))
                    .child(colon)
                    .child(digits(&time.mm))
            };

            // 2. Middle Row: Clock digits + Vinyl disc (if enabled)
            let mut center_row = row().center_y().gap(28.0 * k).child(time_block);

            if s.show_vinyl {
                center_row = center_row.child(depth::vinyl_disc(k, &track.lyrics, accent));
            }

            root = root.child(center_row);

            // 3. Bottom Date Line (if enabled)
            if s.show_date {
                root = root.child(
                    row()
                        .center_y()
                        .gap(16.0 * k)
                        .child(el().size(96.0 * k, 1.0).fill(time.hairline))
                        .child(
                            el()
                                .size(time.date.points.0, time.date.points.1)
                                .pixels(&time.date.pixels),
                        )
                        .child(el().size(96.0 * k, 1.0).fill(time.hairline)),
                );
            }
        }
        None => {
            let fallback = TextStyle::new(faces::TIME_PT * k, 700.0)
                .family(Family::Named("Oxanium"))
                .features("'tnum' 1");
            root = root.child(
                text(format!("{}:{}", s.hh, s.mm))
                    .font(fallback)
                    .color(Color { r: 0.95, g: 0.96, b: 0.98, a: 0.88 }),
            );
        }
    }

    root
}

fn resize(s: &State, tx: &mut Tx) {
    tx.resize_window(s.window);
}

fn poll(s: &mut State, _: f64, tx: &mut Tx) -> bool {
    let mut dirty = false;

    let system_dark = SYSTEM_DARK.load(Ordering::Relaxed);
    if system_dark != s.system_dark {
        s.system_dark = system_dark;
        build(s);
        dirty = true;
    }

    if let Some(input) = INPUT.get() {
        let wanted = input.scale();
        if (wanted - s.scale).abs() > 0.001 {
            s.scale = wanted;
            build(s);
            resize(s, tx);
            dirty = true;
        }
        if input.take_reload() {
            let saved = settings::load();
            s.face = saved.face;
            s.tint = saved.tint;
            s.opacity = saved.opacity;
            s.depth_enabled = saved.depth_enabled;
            s.show_date = saved.show_date;
            s.show_vinyl = saved.show_vinyl;
            s.show_island = saved.show_island;

            if saved.hour24 != s.hour24 {
                s.hour24 = saved.hour24;
                let now = win::now();
                let (hh, mm, meridiem) = clock_parts(&now, s.hour24);
                s.hh = hh;
                s.mm = mm;
                s.date = date_line(&now, meridiem, s.hour24);
            }
            build(s);
            resize(s, tx);
            dirty = true;
        }
        if input.take_reset() {
            let (screen_w, screen_h) = win::screen_size();
            let width = (s.window.0 * dpi()) as i32;
            let height = (s.window.1 * dpi()) as i32;
            if let Some(handle) = WINDOW.lock().ok().and_then(|held| held.clone()) {
                handle.set_position(((screen_w - width) / 2, (screen_h - height) / 2));
            }
            dirty = true;
        }
    }

    let now = win::now();
    let minute = now.hour * 60 + now.minute;
    if minute != s.minute {
        s.minute = minute;
        let (hh, mm, meridiem) = clock_parts(&now, s.hour24);
        s.hh = hh;
        s.mm = mm;
        s.date = date_line(&now, meridiem, s.hour24);
        build(s);
        s.fade = 0.0;
        s.fade_pending = true;
        tx.wake_after(Duration::ZERO);
        dirty = true;
    } else if s.fade_pending {
        s.fade = 1.0;
        s.fade_pending = false;
        dirty = true;
    }

    tx.wake_after(Duration::from_secs(1));
    dirty
}

fn spec(size: (f32, f32)) -> AppSpec<State> {
    AppSpec {
        fonts: glyphs::FONT_BYTES,
        families: ("Oxanium", "JetBrains Mono"),
        poll: Some(poll),
        min_size: Some((1.0, 1.0)),
        ..AppSpec::new("clockscreen", size, init, view)
    }
}

fn window_options(physical: (u32, u32), position: (i32, i32)) -> WindowOptions {
    WindowOptions {
        transparent: true,
        borderless: true,
        click_through: true,
        always_on_top: false,
        visible: true,
        position: Some(position),
        physical_size: Some(physical),
        min_size: None,
        max_size: None,
        resizable: false,
    }
}

fn tray_icon() -> rune::TrayIcon {
    let size = 32u32;
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let centre = (size as f32 - 1.0) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - centre;
            let dy = y as f32 - centre;
            let radius = (dx * dx + dy * dy).sqrt();
            let ring = (12.0..=15.0).contains(&radius);
            let hour_hand = radius < 12.0 && dx.abs() < 1.3 && dy < 0.0;
            let minute_hand = radius < 12.0 && dy.abs() < 1.3 && dx > 0.0;
            let pin = radius < 2.0;
            let alpha = if ring || hour_hand || minute_hand || pin { 255 } else { 0 };
            let at = ((y * size + x) * 4) as usize;
            pixels[at] = 255;
            pixels[at + 1] = 255;
            pixels[at + 2] = 255;
            pixels[at + 3] = alpha;
        }
    }
    rune::TrayIcon::rgba(size, size, pixels)
}

fn install_tray() {
    let menu = vec![
        rune::TrayItem::action("settings", "Studio Settings…"),
        rune::TrayItem::Separator,
        rune::TrayItem::action("face", "Next Face"),
        rune::TrayItem::action("startup", "Start with Windows"),
        rune::TrayItem::Separator,
        rune::TrayItem::action("reset", "Reset Position"),
        rune::TrayItem::action("quit", "Quit"),
    ];
    let spec = rune::TraySpec::new("clockscreen", "ClockScreen", tray_icon())
        .tooltip("ClockScreen")
        .menu(menu);
    match rune::Tray::new(spec, |event| {
        if let rune::TrayEvent::Menu(id) = event {
            match id.as_str() {
                "settings" => panel::open(),
                "face" => {
                    let mut saved = settings::load();
                    saved.face = saved.face.next();
                    settings::save(&saved);
                    request_clock_reload();
                    request_panel_reload();
                }
                "startup" => {
                    let on = !win::start_with_windows();
                    if !win::set_start_with_windows(on) {
                        eprintln!("clockscreen: could not write the Run key");
                    }
                    request_panel_reload();
                }
                "reset" => request_clock_reset(),
                "quit" => std::process::exit(0),
                _ => {}
            }
        }
    }) {
        Ok(tray) => std::mem::forget(tray),
        Err(error) => eprintln!("clockscreen: no tray icon: {error}"),
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("clockscreen {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let saved = settings::load();
    let input = Arc::new(win::Input::new(saved.scale));
    let _ = INPUT.set(input.clone());

    install_tray();

    rune::run_windows(move |displays| {
        let display = displays.iter().find(|display| display.primary).or_else(|| displays.first());
        let scale = display.map(|display| display.scale as f32).unwrap_or(1.0);
        let _ = DPI.set(if scale.is_finite() && scale > 0.05 { scale } else { 1.0 });

        let probe = init();
        let points = probe.window;
        let physical =
            ((points.0 * dpi()).ceil() as u32, (points.1 * dpi()).ceil() as u32);
        let position = match (saved.x, saved.y) {
            (Some(x), Some(y)) => (x, y),
            _ => match display {
                Some(display) => (
                    display.position.0 + (display.size.0 as i32 - physical.0 as i32) / 2,
                    display.position.1 + (display.size.1 as i32 - physical.1 as i32) / 2,
                ),
                None => (0, 0),
            },
        };

        vec![
            WindowSpec::new("clockscreen", spec(points))
                .options(window_options(physical, position))
                .on_created(move |window| {
                    if let Some(hwnd) = win::hwnd_of(&window) {
                        win::pin_to_desktop(hwnd);
                        win::watch_mouse(hwnd, input.clone());
                    }
                    if let Ok(mut held) = WINDOW.lock() {
                        *held = Some(window);
                    }
                })
                .boxed(),
            WindowSpec::new("settings", panel::spec())
                .options(panel::options())
                .on_created(panel::remember)
                .boxed(),
        ]
    });
}
