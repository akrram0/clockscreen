//! The settings window.
//!
//! A second, ordinary Rune window: the clock's own window is transparent, click-through and
//! borderless, which is exactly wrong for controls. This one is decorated, focusable and
//! resizable-less, and it is created hidden at startup so opening it later is instant.
//!
//! It owns no shared state. Every change is written to `settings.json` and announced through
//! [`crate::request_clock_reload`], which is the same path the tray already uses, so the clock
//! and the panel cannot disagree about anything that came from disk.

use rune::tok::{self, Theme, motion, radius, space, type_};
use rune::{AppSpec, Color, Cx, Drag, El, Phase, col, el, row, text, WindowHandle};

use crate::settings::{self, Face, Settings, Tint, OPACITY_MAX, OPACITY_MIN, SCALE_MAX, SCALE_MIN};
use crate::win;

/// The size the panel is created at, in points.
const PANEL_W: f32 = 480.0;
const PANEL_H: f32 = 700.0;

pub struct Panel {
    /// The working copy. Saved on every change, so there is no Cancel.
    draft: Settings,
    start_with_windows: bool,
}

pub fn init() -> Panel {
    Panel { draft: settings::load(), start_with_windows: win::start_with_windows() }
}

/// Persist, then let the clock pick it up on its next poll.
fn commit(p: &mut Panel) {
    settings::save(p.draft);
    crate::request_clock_reload();
}

/// Everything a control needs to paint itself.
#[derive(Clone, Copy)]
struct Paint {
    t: Theme,
    accent: Color,
    hair: f32,
}

pub fn view(p: &Panel, cx: &mut Cx<Panel>) -> El {
    let dark = cx.system_dark();
    let paint =
        Paint { t: tok::theme(dark), accent: tok::accent(0, dark), hair: 1.0 / cx.env().scale };
    let t = paint.t;

    col()
        .key("window")
        .role("window")
        .w_full()
        .h_full()
        .fill(t.bg)
        .child(
            row().h(52.0).w_full().center().px(space::S6).justify_between().child(
                text("ClockScreen")
                    .key("title")
                    .font(type_::TITLE)
                    .color(t.text),
            ),
        )
        .child(
            col()
                .key("content")
                .grow(1.0)
                .w_full()
                .px(space::S6)
                .pb(space::S6)
                .gap(space::S6)
                .child(section(
                    paint,
                    "face",
                    "Clock face",
                    face_grid(paint, p.draft.face),
                ))
                .child(section(
                    paint,
                    "look",
                    "Look",
                    col()
                        .child(setting_row(
                            paint,
                            "tint",
                            "Tint",
                            segmented(
                                paint,
                                "tint-segments",
                                210.0,
                                &[
                                    ("Auto", p.draft.tint == Tint::Auto),
                                    ("Light", p.draft.tint == Tint::Light),
                                    ("Warm", p.draft.tint == Tint::Warm),
                                    ("Cool", p.draft.tint == Tint::Cool),
                                ],
                                |panel, index| {
                                    panel.draft.tint = Tint::ALL[index];
                                    commit(panel);
                                },
                            ),
                        ))
                        .child(separator(paint, "sep-tint"))
                        .child(setting_row(
                            paint,
                            "format",
                            "Clock format",
                            segmented(
                                paint,
                                "format-segments",
                                150.0,
                                &[("24-hour", p.draft.hour24), ("12-hour", !p.draft.hour24)],
                                |panel, index| {
                                    panel.draft.hour24 = index == 0;
                                    commit(panel);
                                },
                            ),
                        ))
                        .child(separator(paint, "sep-format"))
                        .child(slider_row(
                            paint,
                            "opacity",
                            "Opacity",
                            p.draft.opacity,
                            OPACITY_MIN,
                            OPACITY_MAX,
                            |panel, value| panel.draft.opacity = value,
                            |value| format!("{}%", (value * 100.0).round() as i32),
                        ))
                        .child(separator(paint, "sep-opacity"))
                        .child(slider_row(
                            paint,
                            "scale",
                            "Size",
                            p.draft.scale,
                            SCALE_MIN,
                            SCALE_MAX,
                            |panel, value| panel.draft.scale = value,
                            |value| format!("{value:.2}×"),
                        ))
                        .child(
                            el()
                                .key("scale-hint")
                                .px(space::S3)
                                .pb(space::S2)
                                .child(
                                    text("Ctrl + mouse wheel over the clock does the same.")
                                        .font(type_::LABEL)
                                        .color(t.text_secondary),
                                ),
                        ),
                ))
                .child(section(
                    paint,
                    "windows",
                    "Windows",
                    col()
                        .child(toggle_row(
                            paint,
                            "startup",
                            "Start with Windows",
                            p.start_with_windows,
                            |panel| {
                                panel.start_with_windows = !panel.start_with_windows;
                                if !win::set_start_with_windows(panel.start_with_windows) {
                                    eprintln!("clockscreen: could not write the Run key");
                                }
                            },
                        ))
                        .child(separator(paint, "sep-startup"))
                        .child(
                            row()
                                .key("reset")
                                .h(44.0)
                                .px(space::S3)
                                .center_y()
                                .radius(radius::MD)
                                .hover(t.hover)
                                .press(t.press)
                                .focusable(true)
                                .on_press(|_: &mut Panel| crate::request_clock_reset())
                                .child(
                                    text("Reset the clock to the centre of the screen")
                                        .font(type_::BODY)
                                        .color(t.text),
                                ),
                        ),
                )),
        )
}

fn section(paint: Paint, key: &str, title: &str, body: El) -> El {
    let t = paint.t;
    col().key(key).gap(space::S2).child(
        text(title)
            .key("header")
            .font(type_::SECTION)
            .color(t.text_secondary)
            .ml(space::S3),
    ).child(
        col()
            .padding(space::S1)
            .fill(t.card)
            .radius(radius::LG)
            .border(paint.hair, t.card_border)
            .shadow(1.0, 2.0, t.shadow)
            .shadow(8.0, 24.0, t.shadow_soft)
            .child(body),
    )
}

fn separator(paint: Paint, key: &str) -> El {
    el().key(key).h(paint.hair).ml(space::S3).mr(space::S3).fill(paint.t.separator).a11y_hidden()
}

fn setting_row(paint: Paint, key: &str, label: &str, control: El) -> El {
    row()
        .key(key)
        .h(44.0)
        .px(space::S3)
        .center_y()
        .justify_between()
        .radius(radius::MD)
        .label(label)
        .child(text(label).font(type_::BODY).color(paint.t.text).a11y_hidden())
        .child(control)
}

fn toggle_row(
    paint: Paint,
    key: &str,
    label: &str,
    on: bool,
    flip: fn(&mut Panel),
) -> El {
    let t = paint.t;
    let toggle = el()
        .key("switch")
        .role("switch")
        .size(38.0, 22.0)
        .radius(11.0)
        .fill(if on { paint.accent } else { t.control_off })
        .motion(motion::SNAPPY)
        .child(
            el()
                .key("knob")
                .size(18.0, 18.0)
                .absolute(if on { 38.0 - 18.0 - 2.0 } else { 2.0 }, 2.0)
                .radius(9.0)
                .fill(t.knob)
                .shadow(1.0, 3.0, t.knob_shadow)
                .shadow(0.0, 0.5, t.edge),
        )
        .a11y_hidden();
    setting_row(paint, key, label, toggle)
        .role("switch")
        .checked(on)
        .hover(t.hover)
        .press(t.press)
        .focusable(true)
        .on_press(flip)
}

/// Six faces as a 3 x 2 grid of cards: a list of names would say less than the shapes do.
fn face_grid(paint: Paint, current: Face) -> El {
    let t = paint.t;
    let mut grid = col().key("faces").gap(space::S2);
    for pair in Face::ALL.chunks(2) {
        let mut line = row().gap(space::S2);
        for face in pair.iter().copied() {
            let selected = face == current;
            let i = Face::ALL.iter().position(|other| *other == face).unwrap_or(0);
            line = line.child(
                col()
                    .key(face.label())
                    .role("radio")
                    .label(face.label())
                    .checked(selected)
                    .focusable(true)
                    .size(196.0, 52.0)
                    .padding(space::S2)
                    .gap(2.0)
                    .radius(radius::MD)
                    .fill(if selected { t.control_track } else { Color::TRANSPARENT })
                    .border(paint.hair, if selected { paint.accent } else { t.card_border })
                    .hover(t.hover)
                    .press(t.press)
                    .on_press(move |panel: &mut Panel| {
                        panel.draft.face = Face::ALL[i];
                        commit(panel);
                    })
                    .child(
                        text(face.label())
                            .font(type_::BODY)
                            .color(if selected { t.text } else { t.text_secondary })
                            .a11y_hidden(),
                    )
                    .child(
                        text(face.hint())
                            .font(type_::LABEL)
                            .color(t.text_secondary)
                            .a11y_hidden(),
                    )
                    .child(if selected {
                        el()
                            .size(6.0, 6.0)
                            .absolute(196.0 - 16.0, 10.0)
                            .radius(3.0)
                            .fill(paint.accent)
                            .a11y_hidden()
                    } else {
                        el().size(0.0, 0.0).a11y_hidden()
                    }),
            );
        }
        grid = grid.child(line);
    }
    grid
}

/// A segmented control: `width` points wide, one cell per item, a sliding indicator.
fn segmented(
    paint: Paint,
    key: &'static str,
    width: f32,
    items: &[(&'static str, bool)],
    pick: fn(&mut Panel, usize),
) -> El {
    let t = paint.t;
    let (h, pad) = (28.0, 2.0);
    let count = items.len().max(1);
    let seg = (width - 2.0 * pad) / count as f32;
    let selected = items.iter().position(|(_, on)| *on).unwrap_or(0);

    let mut cells = row().key("segments");
    for (index, (name, on)) in items.iter().copied().enumerate() {
        cells = cells.child(
            el()
                .key(name)
                .role("segment")
                .label(name)
                .checked(on)
                .focusable(true)
                .size(seg, h - 2.0 * pad)
                .center()
                .radius(radius::SM)
                .hover(if on { Color::TRANSPARENT } else { t.hover })
                .on_press(move |panel: &mut Panel| {
                    pick(panel, index);
                })
                .child(
                    text(name)
                        .font(type_::LABEL)
                        .color(if on { t.text } else { t.text_secondary })
                        .a11y_hidden(),
                ),
        );
    }

    el()
        .key(key)
        .role("segmented")
        .size(width, h)
        .padding(pad)
        .radius(radius::MD)
        .fill(t.control_track)
        .motion(motion::SNAPPY)
        .child(
            el()
                .key("indicator")
                .size(seg, h - 2.0 * pad)
                .absolute(pad + selected as f32 * seg, pad)
                .radius(radius::SM)
                .fill(t.indicator)
                .shadow(1.0, 3.0, t.shadow)
                .shadow(0.0, 0.5, t.edge)
                .lead(motion::LEAD, motion::DEFAULT)
                .a11y_hidden(),
        )
        .child(cells)
}

fn slider_row(
    paint: Paint,
    key: &'static str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    set: fn(&mut Panel, f32),
    show: fn(f32) -> String,
) -> El {
    let t = paint.t;
    let (w, h, knob, bar) = (180.0, 22.0, 18.0, 4.0);
    let travel = w - knob;
    let span = (max - min).max(f32::EPSILON);
    let norm = ((value - min) / span).clamp(0.0, 1.0);
    let to_value = move |x: f32| min + ((x - knob / 2.0) / travel).clamp(0.0, 1.0) * span;

    let track = row()
        .key("slider-control")
        .gap(space::S3)
        .center_y()
        .child(
            el()
                .key("slider")
                .role("slider")
                .label(label)
                .range_value(f64::from(norm), 0.0, 1.0)
                .range_step(0.01)
                .a11y_value(show(value))
                .on_step(move |panel: &mut Panel, steps, _| {
                    set(panel, (norm + steps as f32 * 0.01).clamp(0.0, 1.0) * span + min);
                    commit(panel);
                })
                .size(w, h)
                .motion(motion::SNAPPY)
                .on_drag(move |panel: &mut Panel, drag: &mut Drag, tx| match drag.phase {
                    Phase::Down => {
                        drag.data = norm;
                        let grab = knob / 2.0 + norm * travel;
                        if (drag.x - grab).abs() > knob / 2.0 + 2.0 {
                            // Pressed the track, not the knob: jump there.
                            set(panel, to_value(drag.x));
                            commit(panel);
                        }
                    }
                    Phase::Move => {
                        let grab = knob / 2.0 + drag.data * travel;
                        let offset =
                            if (drag.start_x - grab).abs() <= knob / 2.0 + 2.0 { drag.start_x - grab } else { 0.0 };
                        set(panel, to_value(drag.x - offset));
                        commit(panel);
                        tx.instant();
                    }
                    Phase::Up => {
                        if drag.cancelled() {
                            set(panel, min + drag.data * span);
                            commit(panel);
                        }
                    }
                })
                .child(
                    el()
                        .key("track")
                        .size(w, bar)
                        .absolute(0.0, (h - bar) / 2.0)
                        .radius(bar / 2.0)
                        .fill(t.control_off)
                        .a11y_hidden(),
                )
                .child(
                    el()
                        .key("fill")
                        .size(knob / 2.0 + norm * travel, bar)
                        .absolute(0.0, (h - bar) / 2.0)
                        .radius(bar / 2.0)
                        .fill(paint.accent)
                        .a11y_hidden(),
                )
                .child(
                    el()
                        .key("knob")
                        .size(knob, knob)
                        .absolute(norm * travel, (h - knob) / 2.0)
                        .radius(knob / 2.0)
                        .fill(t.knob)
                        .shadow(1.0, 3.0, t.knob_shadow)
                        .shadow(0.0, 0.5, t.edge)
                        .a11y_hidden(),
                ),
        )
        .child(
            text(show(value))
                .key("value")
                .w(44.0)
                .font(type_::MONO_CAPTION)
                .color(t.text_secondary)
                .align(rune::Align::End)
                .a11y_hidden(),
        );

    setting_row(paint, key, label, track)
}

/// The panel follows the file: the tray can change the face while the panel is closed.
pub fn poll(p: &mut Panel, _: f64, _: &mut rune::Tx) -> bool {
    if !crate::take_panel_reload() {
        return false;
    }
    p.draft = settings::load();
    p.start_with_windows = win::start_with_windows();
    true
}

pub fn spec() -> AppSpec<Panel> {
    AppSpec { poll: Some(poll), ..AppSpec::new("ClockScreen settings", (PANEL_W, PANEL_H), init, view) }
}

/// Created hidden at startup; `open()` shows it, and closing it only closes the panel.
pub fn options() -> rune::WindowOptions {
    rune::WindowOptions {
        visible: false,
        resizable: false,
        max_size: Some((PANEL_W, PANEL_H)),
        ..rune::WindowOptions::default()
    }
}

static HANDLE: std::sync::Mutex<Option<WindowHandle>> = std::sync::Mutex::new(None);

pub fn remember(handle: WindowHandle) {
    if let Ok(mut held) = HANDLE.lock() {
        *held = Some(handle);
    }
}

/// Show and focus the panel, refreshing it from disk first.
pub fn open() {
    crate::request_panel_reload();
    if let Some(handle) = HANDLE.lock().ok().and_then(|held| held.clone()) {
        handle.open();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_has_a_hint() {
        for face in Face::ALL {
            assert!(!face.hint().is_empty(), "{face:?} needs a one-line description");
            assert!(face.hint().len() < 32, "{face:?}'s hint is too long for the card");
        }
    }

    #[test]
    fn a_slider_round_trips_its_ends() {
        let min = OPACITY_MIN;
        let max = OPACITY_MAX;
        let span = max - min;
        // The normalised position of each end, then back again.
        for value in [min, (min + max) / 2.0, max] {
            let norm = ((value - min) / span).clamp(0.0, 1.0);
            let back = min + norm * span;
            assert!((back - value).abs() < 1e-6, "{value} became {back}");
        }
    }
}
