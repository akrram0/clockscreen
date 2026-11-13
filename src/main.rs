//! ClockScreen: `HH:MM` on the wallpaper, nothing else.
//!
//! Rune draws the text; Win32 puts the window at the bottom of the z-order, keeps it out of the
//! taskbar, and watches the mouse so the clock can be dragged and scaled while it stays
//! click-through.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod settings;
mod win;

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use rune::{
    col, el, row, text, AppSpec, Color, Cx, El, Family, LoopSpec, Spring, TextStyle, Tx,
    WindowHandle, WindowOptions, WindowSpec,
};

const BASE_W: f32 = 780.0;
const BASE_H: f32 = 400.0;
/// The time's cap height at scale 1, near enough to the 220 px the design asks for.
const TIME_PT: f32 = 220.0;
const DATE_PT: f32 = 30.0;
const FAMILY: &str = "Chakra Petch";

static FONTS: &[&[u8]] = &[include_bytes!("../assets/ChakraPetch-Bold.ttf")];

static INPUT: OnceLock<Arc<win::Input>> = OnceLock::new();
static WINDOW: Mutex<Option<WindowHandle>> = Mutex::new(None);

struct State {
    hh: String,
    mm: String,
    date: String,
    minute: u32,
    scale: f32,
    fade: f32,
    fade_pending: bool,
}

fn two(n: u32) -> String {
    format!("{n:02}")
}

const WEEKDAYS: [&str; 7] =
    ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

fn date_line(now: &win::Now) -> String {
    let weekday = WEEKDAYS.get(now.weekday as usize).copied().unwrap_or("");
    let month = MONTHS.get(now.month.saturating_sub(1) as usize).copied().unwrap_or("");
    format!("{weekday} {} {month}", now.day)
}

fn init() -> State {
    let now = win::now();
    let scale = INPUT
        .get()
        .map(|input| input.scale())
        .unwrap_or_else(|| settings::load().scale);
    State {
        hh: two(now.hour),
        mm: two(now.minute),
        date: date_line(&now),
        minute: now.hour * 60 + now.minute,
        scale,
        fade: 1.0,
        fade_pending: false,
    }
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let k = s.scale;
    // Rune paints a text with one solid colour: there is no gradient or stroke inside glyphs.
    // Translucency is what makes the wallpaper read through, so the face is near-white at 88%.
    let face = Color { r: 0.95, g: 0.96, b: 0.98, a: 0.88 };
    let date_ink = Color { r: 0.95, g: 0.96, b: 0.98, a: 0.60 };
    let hairline = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.22 };

    let time_style = || {
        TextStyle::new(TIME_PT * k, 700.0)
            .family(Family::Named(FAMILY))
            .tracking(2.0 * k)
    };
    let date_style = || {
        TextStyle::new(DATE_PT * k, 300.0)
            .family(Family::Named(FAMILY))
            .tracking(6.5 * k)
    };
    let settle = || Spring::new(0.22, 0.45);

    col()
        .w_full()
        .h_full()
        .center()
        .gap(10.0 * k)
        .child(
            row()
                .center_y()
                .child(
                    text(s.hh.as_str())
                        .font(time_style())
                        .color(face)
                        .opacity(s.fade)
                        .opacity_motion(settle()),
                )
                .child(
                    text(":")
                        .font(time_style())
                        .color(face)
                        // Triangle, ease-in-out: a soft fade twice per period, so one blink a second.
                        .looping(LoopSpec::opacity(1.0, 0.0, 0.18, 1.0)),
                )
                .child(
                    text(s.mm.as_str())
                        .font(time_style())
                        .color(face)
                        .opacity(s.fade)
                        .opacity_motion(settle()),
                ),
        )
        .child(
            row()
                .center_y()
                .gap(18.0 * k)
                .child(el().size(96.0 * k, 1.0).fill(hairline))
                .child(text(s.date.as_str()).font(date_style()).color(date_ink))
                .child(el().size(96.0 * k, 1.0).fill(hairline)),
        )
}

fn poll(s: &mut State, _: f64, tx: &mut Tx) -> bool {
    let mut dirty = false;

    if let Some(input) = INPUT.get() {
        let wanted = input.scale();
        if (wanted - s.scale).abs() > 0.001 {
            s.scale = wanted;
            tx.resize_window((BASE_W * wanted, BASE_H * wanted));
            dirty = true;
        }
        if input.take_reset() {
            let (screen_w, screen_h) = win::screen_size();
            let width = (BASE_W * s.scale) as i32;
            let height = (BASE_H * s.scale) as i32;
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
        s.hh = two(now.hour);
        s.mm = two(now.minute);
        s.date = date_line(&now);
        // Drop to nothing, then let the spring bring the new digits back.
        s.fade = 0.0;
        s.fade_pending = true;
        dirty = true;
    } else if s.fade_pending {
        s.fade = 1.0;
        s.fade_pending = false;
        dirty = true;
    }

    tx.wake_after(Duration::from_secs(1));
    dirty
}

fn spec() -> AppSpec<State> {
    AppSpec {
        fonts: FONTS,
        families: (FAMILY, "Inter"),
        poll: Some(poll),
        min_size: Some((BASE_W * settings::SCALE_MIN, BASE_H * settings::SCALE_MIN)),
        ..AppSpec::new("clockscreen", (BASE_W, BASE_H), init, view)
    }
}

fn window_options(saved: &settings::Settings) -> WindowOptions {
    WindowOptions {
        transparent: true,
        borderless: true,
        click_through: true,
        always_on_top: false,
        visible: true,
        position: match (saved.x, saved.y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        },
        physical_size: None,
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
        rune::TrayItem::action("startup", "Start with Windows"),
        rune::TrayItem::Separator,
        rune::TrayItem::action("reset", "Reset position"),
        rune::TrayItem::action("quit", "Quit"),
    ];
    let spec = rune::TraySpec::new("clockscreen", "ClockScreen", tray_icon())
        .tooltip("ClockScreen")
        .menu(menu);
    match rune::Tray::new(spec, |event| {
        if let rune::TrayEvent::Menu(id) = event {
            match id.as_str() {
                "startup" => {
                    let on = !win::start_with_windows();
                    if !win::set_start_with_windows(on) {
                        eprintln!("clockscreen: could not write the Run key");
                    }
                }
                "reset" => {
                    if let Some(input) = INPUT.get() {
                        input.request_reset();
                    }
                }
                "quit" => std::process::exit(0),
                _ => {}
            }
        }
    }) {
        // The tray lives for the whole process; leaking it is what keeps the icon up.
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
    let shared = Arc::new(Mutex::new(saved));
    let _ = INPUT.set(input.clone());

    install_tray();

    rune::run_windows(move |_displays| {
        vec![WindowSpec::new("clockscreen", spec())
            .options(window_options(&saved))
            .on_created(move |window| {
                if let Some(hwnd) = win::hwnd_of(&window) {
                    win::pin_to_desktop(hwnd);
                    win::watch_mouse(hwnd, input.clone(), shared.clone());
                }
                if let Ok(mut held) = WINDOW.lock() {
                    *held = Some(window.clone());
                }
            })]
    });
}
