//! The settings window / Desktop Management App.
//!
//! A second, ordinary Rune window: the clock's own window is transparent, click-through and
//! borderless, which is exactly wrong for controls. This one is decorated, focusable,
//! and it is created hidden at startup so opening it later is instant.
//!
//! It owns no shared state. Every change is written to `settings.json` and announced through
//! [`crate::request_clock_reload`], which is the same path the tray already uses, so the clock
//! and the panel cannot disagree about anything that came from disk.

use rune::tok::{self, motion, radius, space, type_, Theme};
use rune::{col, el, row, text, AppSpec, Color, Cx, Drag, El, Phase, WindowHandle};

use crate::settings::{self, Face, Settings, Tint, OPACITY_MAX, OPACITY_MIN, SCALE_MAX, SCALE_MIN};
use crate::win;

/// The size the panel is created at, in points.
const PANEL_W: f32 = 500.0;
const PANEL_H: f32 = 860.0;

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
    settings::save(&p.draft);
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
            row().h(56.0).w_full().center().px(space::S6).justify_between().child(
                text("ClockScreen Studio")
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
                    "Clock Face",
                    face_grid(paint, p.draft.face),
                ))
                .child(section(
                    paint,
                    "depth",
                    "Depth Effect & Wallpaper",
                    col()
                        .child(toggle_row(
                            paint,
                            "depth-enabled",
                            "Wallpaper Depth Occlusion",
                            p.draft.depth_enabled,
                            |panel| {
                                panel.draft.depth_enabled = !panel.draft.depth_enabled;
                                commit(panel);
                            },
                        ))
                        .child(
                            el()
                                .key("depth-hint")
                                .px(space::S3)
                                .pb(space::S2)
                                .child(
                                    text("Tucks the clock behind foreground subjects (like fish/hair).")
                                        .font(type_::LABEL)
                                        .color(t.text_secondary),
                                ),
                        ),
                ))
                .child(section(
                    paint,
                    "widgets",
                    "Companion Widgets",
                    col()
                        .child(toggle_row(
                            paint,
                            "toggle-island",
                            "Top Dynamic Media Island",
                            p.draft.show_island,
                            |panel| {
                                panel.draft.show_island = !panel.draft.show_island;
                                commit(panel);
                            },
                        ))
                        .child(separator(paint, "sep-w1"))
                        .child(toggle_row(
                            paint,
                            "toggle-vinyl",
                            "Circular Vinyl Visualizer Disc",
                            p.draft.show_vinyl,
                            |panel| {
                                panel.draft.show_vinyl = !panel.draft.show_vinyl;
                                commit(panel);
                            },
                        ))
                        .child(separator(paint, "sep-w2"))
                        .child(toggle_row(
                            paint,
                            "toggle-date",
                            "Bottom Date Line",
                            p.draft.show_date,
                            |panel| {
                                panel.draft.show_date = !panel.draft.show_date;
                                commit(panel);
                            },
                        )),
                ))
                .child(section(
                    paint,
                    "look",
                    "Typography & Appearance",
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
                                    text("Hold mouse on clock to drag; Ctrl + wheel scales.")
                                        .font(type_::LABEL)
                                        .color(t.text_secondary),
                                ),
                        ),
                ))
                .child(section(
                    paint,
                    "windows",
                    "System",
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
                                    text("Reset position to screen centre")
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
    )
    .child(
        col()
            .key("card")
            .w_full()
            .radius(radius::LG)
            .fill(t.card)
            .border(paint.hair, t.border)
            .p(space::S2)
            .child(body),
    )
}

fn setting_row(paint: Paint, key: &str, label: &str, control: El) -> El {
    let t = paint.t;
    row()
        .key(key)
        .h(44.0)
        .px(space::S3)
        .center_y()
        .justify_between()
        .child(text(label).font(type_::BODY).color(t.text))
        .child(control)
}

fn separator(paint: Paint, key: &str) -> El {
    el().key(key).h(paint.hair).w_full().fill(paint.t.border).my(space::S1)
}

fn face_grid(paint: Paint, selected: Face) -> El {
    let mut grid = col().key("face-grid").gap(space::S2);
    for chunk in Face::ALL.chunks(3) {
        let mut r = row().gap(space::S2).w_full();
        for &face in chunk {
            r = r.child(face_cell(paint, face, face == selected));
        }
        grid = grid.child(r);
    }
    grid
}

fn face_cell(paint: Paint, face: Face, is_selected: bool) -> El {
    let t = paint.t;
    let base = col()
        .key(face.id())
        .role("radio")
        .a11y_selected(is_selected)
        .grow(1.0)
        .h(64.0)
        .p(space::S3)
        .radius(radius::MD)
        .justify_between()
        .focusable(true)
        .on_press(move |panel: &mut Panel| {
            panel.draft.face = face;
            commit(panel);
        });

    let styled = if is_selected {
        base.fill(paint.accent)
            .shadow(0.0, 4.0, t.press)
            .child(
                text(face.label())
                    .font(type_::HEADING)
                    .color(Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
            )
            .child(
                text(face.hint())
                    .font(type_::LABEL)
                    .color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.85 }),
            )
    } else {
        base.hover(t.hover)
            .press(t.press)
            .child(text(face.label()).font(type_::HEADING).color(t.text))
            .child(text(face.hint()).font(type_::LABEL).color(t.text_secondary))
    };

    styled
}

fn segmented<F>(paint: Paint, key: &str, width: f32, options: &[(&str, bool)], on_pick: F) -> El
where
    F: Fn(&mut Panel, usize) + 'static + Copy,
{
    let t = paint.t;
    let mut container = row()
        .key(key)
        .w(width)
        .h(32.0)
        .radius(radius::MD)
        .fill(t.control_off)
        .p(2.0)
        .gap(2.0);

    for (index, &(label, is_on)) in options.iter().enumerate() {
        let mut button = row()
            .key(label)
            .grow(1.0)
            .h_full()
            .center()
            .radius(radius::SM)
            .on_press(move |panel: &mut Panel| on_pick(panel, index));

        if is_on {
            button = button.fill(t.card).shadow(0.0, 2.0, t.edge).child(
                text(label)
                    .font(type_::LABEL)
                    .color(t.text),
            );
        } else {
            button = button.hover(t.hover).child(
                text(label)
                    .font(type_::LABEL)
                    .color(t.text_secondary),
            );
        }
        container = container.child(button);
    }
    container
}

fn toggle_row<F>(paint: Paint, key: &str, label: &str, on: bool, on_toggle: F) -> El
where
    F: Fn(&mut Panel) + 'static + Copy,
{
    let t = paint.t;
    let w = 42.0;
    let h = 24.0;
    let knob = 18.0;
    let margin = 3.0;

    let switch = row()
        .key("switch")
        .size(w, h)
        .radius(h / 2.0)
        .fill(if on { paint.accent } else { t.control_off })
        .p(margin)
        .on_press(move |panel: &mut Panel| on_toggle(panel))
        .child(
            el()
                .size(knob, knob)
                .radius(knob / 2.0)
                .fill(t.knob)
                .shadow(0.0, 2.0, t.knob_shadow)
                .ml(if on { w - knob - margin * 2.0 } else { 0.0 }),
        );

    setting_row(paint, key, label, switch)
}

fn slider_row<F, S>(
    paint: Paint,
    key: &str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    set: F,
    show: S,
) -> El
where
    F: Fn(&mut Panel, f32) + 'static + Copy,
    S: Fn(f32) -> String + 'static + Copy,
{
    let t = paint.t;
    let w = 180.0;
    let h = 28.0;
    let bar = 4.0;
    let knob = 18.0;
    let travel = w - knob;
    let span = max - min;
    let norm = ((value - min) / span).clamp(0.0, 1.0);

    let to_value = move |cursor_x: f32| {
        let pos = (cursor_x - knob / 2.0) / travel;
        (pos.clamp(0.0, 1.0) * span + min).clamp(min, max)
    };

    let track = row()
        .key("slider-container")
        .gap(space::S3)
        .center_y()
        .child(
            el()
                .key("slider")
                .role("slider")
                .range(min, max)
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
    AppSpec { poll: Some(poll), ..AppSpec::new("ClockScreen Studio", (PANEL_W, PANEL_H), init, view) }
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
        for value in [min, (min + max) / 2.0, max] {
            let norm = ((value - min) / span).clamp(0.0, 1.0);
            let back = min + norm * span;
            assert!((back - value).abs() < 1e-6, "{value} became {back}");
        }
    }
}
