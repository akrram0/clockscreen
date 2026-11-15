//! ClockScreen Studio: The Desktop Management App.
//! Designed strictly to Apple/macOS Sonoma dark-glass aesthetic tokens (.agent/rules/design.md).
//! Pure native Win32 with double-buffered rendering for 0% CPU and zero flicker.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, FillRect, FrameRect, GetDC, ReleaseDC, SelectObject, SetBkMode,
    SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, FW_BOLD, FW_REGULAR,
    HDC, HFONT, HGDIOBJ, OUT_DEFAULT_PRECIS, SRCCOPY, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClientRect, GetSystemMetrics, InvalidateRect,
    RegisterClassW, SetForegroundWindow, ShowWindow, DRAW_TEXT_FORMAT, DT_CENTER, DT_LEFT,
    DT_RIGHT, DT_SINGLELINE, DT_VCENTER, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW, WM_CLOSE,
    WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WNDCLASSW,
    WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

use crate::clock_window::{request_clock_reload, request_clock_reset};
use crate::settings::{
    self, set_start_with_windows, start_with_windows, Face, Tint, OPACITY_MAX, OPACITY_MIN,
    SCALE_MAX, SCALE_MIN,
};

static STUDIO_HWND: OnceLock<HWND> = OnceLock::new();
static DRAGGING_SLIDER: OnceLock<AtomicBool> = OnceLock::new();

fn is_dragging_slider() -> &'static AtomicBool {
    DRAGGING_SLIDER.get_or_init(|| AtomicBool::new(false))
}

const COLOR_BG: COLORREF = COLORREF(0x0a | (0x0c << 8) | (0x10 << 16));      // #0a0c10 Deep Canvas
const COLOR_CARD: COLORREF = COLORREF(0x16 | (0x19 << 8) | (0x22 << 16));    // #161922 Glass Surface
const COLOR_BORDER: COLORREF = COLORREF(0x28 | (0x2d << 8) | (0x3c << 16));  // #282d3c Border
const COLOR_ACCENT: COLORREF = COLORREF(0x38 | (0xbd << 8) | (0xf8 << 16));  // #38bdf8 Electric Blue
const COLOR_AMBER: COLORREF = COLORREF(0xf5 | (0x9e << 8) | (0x0b << 16));   // #f59e0b Warm
const COLOR_TEXT_PRI: COLORREF = COLORREF(0xf4 | (0xf6 << 8) | (0xfb << 16));// #f4f6fb Text Primary
const COLOR_TEXT_SEC: COLORREF = COLORREF(0x9b | (0xa3 << 8) | (0xb8 << 16));// #9ba3b8 Text Secondary
const COLOR_TEXT_DIM: COLORREF = COLORREF(0x4a | (0x52 << 8) | (0x68 << 16));// #4a5268 Subdued

pub fn show_studio_window() {
    if let Some(hwnd) = STUDIO_HWND.get().copied() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
    } else {
        unsafe {
            let _ = create_studio_window();
        }
    }
}

pub unsafe fn create_studio_window() -> windows::core::Result<HWND> {
    let instance = GetModuleHandleW(None)?;
    let class_name = w!("ClockScreenStudioClass");

    let wc = WNDCLASSW {
        lpfnWndProc: Some(studio_wnd_proc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    let _ = RegisterClassW(&wc);

    let screen_w = GetSystemMetrics(SM_CXSCREEN);
    let screen_h = GetSystemMetrics(SM_CYSCREEN);
    let win_w = 540;
    let win_h = 760;
    let win_x = (screen_w - win_w) / 2;
    let win_y = (screen_h - win_h) / 2;

    let hwnd = CreateWindowExW(
        windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
        class_name,
        w!("ClockScreen Studio"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        win_x,
        win_y,
        win_w,
        win_h,
        None,
        None,
        instance,
        None,
    )?;

    let dark_mode = windows::Win32::Foundation::BOOL(1);
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        &dark_mode as *const _ as *const _,
        std::mem::size_of_val(&dark_mode) as u32,
    );

    let _ = STUDIO_HWND.set(hwnd);
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);

    Ok(hwnd)
}

fn draw_rounded_rect(hdc: HDC, rect: &RECT, bg: COLORREF, border: COLORREF) {
    unsafe {
        let bg_brush = CreateSolidBrush(bg);
        let border_brush = CreateSolidBrush(border);

        FillRect(hdc, rect, bg_brush);
        FrameRect(hdc, rect, border_brush);

        let _ = DeleteObject(HGDIOBJ(bg_brush.0));
        let _ = DeleteObject(HGDIOBJ(border_brush.0));
    }
}

unsafe fn make_font(height: i32, bold: bool) -> HFONT {
    CreateFontW(
        -height,
        0,
        0,
        0,
        if bold { FW_BOLD.0 as i32 } else { FW_REGULAR.0 as i32 },
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        0,
        w!("Segoe UI"),
    )
}

unsafe fn draw_text(hdc: HDC, text: &str, rect: &mut RECT, format: DRAW_TEXT_FORMAT) {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let _ = DrawTextW(hdc, &mut wide, rect, format);
}

pub unsafe extern "system" fn studio_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1), // Prevent GDI flicker
        WM_PAINT => {
            let mut client = RECT::default();
            GetClientRect(hwnd, &mut client);
            let w = client.right - client.left;
            let h = client.bottom - client.top;

            let screen_dc = GetDC(Some(hwnd));
            let mem_dc = CreateCompatibleDC(Some(screen_dc)).unwrap();
            let h_bmp = CreateCompatibleBitmap(screen_dc, w, h).unwrap();
            let old_bmp = SelectObject(mem_dc, HGDIOBJ(h_bmp.0));

            // Fill canvas:
            let bg_brush = CreateSolidBrush(COLOR_BG);
            FillRect(mem_dc, &client, bg_brush);
            let _ = DeleteObject(HGDIOBJ(bg_brush.0));

            SetBkMode(mem_dc, TRANSPARENT);

            let font_title = make_font(22, true);
            let font_subhead = make_font(14, true);
            let font_body = make_font(13, false);
            let font_caption = make_font(11, false);

            let s = settings::load();

            // 1. HEADER
            SelectObject(mem_dc, HGDIOBJ(font_title.0));
            SetTextColor(mem_dc, COLOR_TEXT_PRI);
            let mut r_title = RECT { left: 24, top: 20, right: 340, bottom: 46 };
            draw_text(mem_dc, "ClockScreen Studio", &mut r_title, DT_LEFT | DT_SINGLELINE);

            SelectObject(mem_dc, HGDIOBJ(font_caption.0));
            SetTextColor(mem_dc, COLOR_TEXT_SEC);
            let mut r_subtitle = RECT { left: 24, top: 48, right: 340, bottom: 66 };
            draw_text(mem_dc, "Desktop Wallpaper Depth Clock Manager", &mut r_subtitle, DT_LEFT | DT_SINGLELINE);

            // Active Badge:
            let r_badge = RECT { left: 390, top: 24, right: 516, bottom: 50 };
            draw_rounded_rect(mem_dc, &r_badge, COLOR_CARD, COLOR_ACCENT);
            SetTextColor(mem_dc, COLOR_ACCENT);
            let mut r_badge_txt = r_badge;
            draw_text(mem_dc, "● Active on Desktop", &mut r_badge_txt, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // 2. FACE CARDS (2x3 grid)
            SelectObject(mem_dc, HGDIOBJ(font_subhead.0));
            SetTextColor(mem_dc, COLOR_TEXT_PRI);
            let mut r_face_sec = RECT { left: 24, top: 82, right: 500, bottom: 104 };
            draw_text(mem_dc, "Clock Face Style", &mut r_face_sec, DT_LEFT | DT_SINGLELINE);

            let faces = Face::ALL;
            for i in 0..6 {
                let col = i % 2;
                let row = i / 2;
                let card_x = 24 + col * 252;
                let card_y = 110 + row * 62;
                let card_rect = RECT { left: card_x, top: card_y, right: card_x + 240, bottom: card_y + 54 };

                let is_active = s.face == faces[i as usize];
                let bg = if is_active { COLORREF(0x1a | (0x2b << 8) | (0x44 << 16)) } else { COLOR_CARD };
                let border = if is_active { COLOR_ACCENT } else { COLOR_BORDER };

                draw_rounded_rect(mem_dc, &card_rect, bg, border);

                SelectObject(mem_dc, HGDIOBJ(font_body.0));
                SetTextColor(mem_dc, if is_active { COLOR_ACCENT } else { COLOR_TEXT_PRI });
                let mut r_name = RECT { left: card_x + 12, top: card_y + 8, right: card_x + 228, bottom: card_y + 26 };
                draw_text(mem_dc, faces[i as usize].label(), &mut r_name, DT_LEFT | DT_SINGLELINE);

                SelectObject(mem_dc, HGDIOBJ(font_caption.0));
                SetTextColor(mem_dc, COLOR_TEXT_SEC);
                let mut r_hint = RECT { left: card_x + 12, top: card_y + 28, right: card_x + 228, bottom: card_y + 46 };
                draw_text(mem_dc, faces[i as usize].hint(), &mut r_hint, DT_LEFT | DT_SINGLELINE);
            }

            // 3. TINT SELECTOR
            let tint_y = 310;
            SelectObject(mem_dc, HGDIOBJ(font_subhead.0));
            SetTextColor(mem_dc, COLOR_TEXT_PRI);
            let mut r_tint_sec = RECT { left: 24, top: tint_y, right: 500, bottom: tint_y + 22 };
            draw_text(mem_dc, "Appearance Tint", &mut r_tint_sec, DT_LEFT | DT_SINGLELINE);

            let tints = Tint::ALL;
            for i in 0..5 {
                let pill_x = 24 + i * 98;
                let pill_rect = RECT { left: pill_x, top: tint_y + 28, right: pill_x + 90, bottom: tint_y + 56 };
                let is_active = s.tint == tints[i as usize];

                let bg = if is_active { COLORREF(0x1a | (0x2b << 8) | (0x44 << 16)) } else { COLOR_CARD };
                let border = if is_active { COLOR_ACCENT } else { COLOR_BORDER };
                draw_rounded_rect(mem_dc, &pill_rect, bg, border);

                SelectObject(mem_dc, HGDIOBJ(font_body.0));
                SetTextColor(mem_dc, if is_active { COLOR_ACCENT } else { COLOR_TEXT_PRI });
                let mut r_txt = pill_rect;
                draw_text(mem_dc, tints[i as usize].label(), &mut r_txt, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            }

            // 4. SLIDERS: Scale and Opacity
            let slider_y = 388;
            SelectObject(mem_dc, HGDIOBJ(font_body.0));
            SetTextColor(mem_dc, COLOR_TEXT_PRI);
            let mut r_lbl_scale = RECT { left: 24, top: slider_y, right: 160, bottom: slider_y + 20 };
            draw_text(mem_dc, "Clock Scale", &mut r_lbl_scale, DT_LEFT | DT_SINGLELINE);

            let scale_val_txt = format!("{:.2}×", s.scale);
            let mut r_val_scale = RECT { left: 450, top: slider_y, right: 516, bottom: slider_y + 20 };
            draw_text(mem_dc, &scale_val_txt, &mut r_val_scale, DT_RIGHT | DT_SINGLELINE);

            let r_track_scale = RECT { left: 24, top: slider_y + 24, right: 516, bottom: slider_y + 34 };
            draw_rounded_rect(mem_dc, &r_track_scale, COLOR_CARD, COLOR_BORDER);
            let scale_pct = (s.scale - SCALE_MIN) / (SCALE_MAX - SCALE_MIN);
            let thumb_x = 24 + (scale_pct * (516.0 - 24.0)) as i32;
            let r_thumb_scale = RECT { left: thumb_x - 8, top: slider_y + 19, right: thumb_x + 8, bottom: slider_y + 39 };
            draw_rounded_rect(mem_dc, &r_thumb_scale, COLOR_ACCENT, COLOR_TEXT_PRI);

            // Opacity Slider:
            let op_y = slider_y + 46;
            let mut r_lbl_op = RECT { left: 24, top: op_y, right: 160, bottom: op_y + 20 };
            draw_text(mem_dc, "Opacity", &mut r_lbl_op, DT_LEFT | DT_SINGLELINE);

            let op_val_txt = format!("{}%", (s.opacity * 100.0).round() as i32);
            let mut r_val_op = RECT { left: 450, top: op_y, right: 516, bottom: op_y + 20 };
            draw_text(mem_dc, &op_val_txt, &mut r_val_op, DT_RIGHT | DT_SINGLELINE);

            let r_track_op = RECT { left: 24, top: op_y + 24, right: 516, bottom: op_y + 34 };
            draw_rounded_rect(mem_dc, &r_track_op, COLOR_CARD, COLOR_BORDER);
            let op_pct = (s.opacity - OPACITY_MIN) / (OPACITY_MAX - OPACITY_MIN);
            let thumb_op_x = 24 + (op_pct * (516.0 - 24.0)) as i32;
            let r_thumb_op = RECT { left: thumb_op_x - 8, top: op_y + 19, right: thumb_op_x + 8, bottom: op_y + 39 };
            draw_rounded_rect(mem_dc, &r_thumb_op, COLOR_ACCENT, COLOR_TEXT_PRI);

            // 5. TOGGLES & ACTIONS (Cards with checkboxes)
            let act_y = 480;
            SelectObject(mem_dc, HGDIOBJ(font_subhead.0));
            SetTextColor(mem_dc, COLOR_TEXT_PRI);
            let mut r_opt_sec = RECT { left: 24, top: act_y, right: 500, bottom: act_y + 22 };
            draw_text(mem_dc, "Desktop Options", &mut r_opt_sec, DT_LEFT | DT_SINGLELINE);

            // Toggle 1: Lock Position (Click-Through)
            let r_btn_lock = RECT { left: 24, top: act_y + 28, right: 260, bottom: act_y + 66 };
            draw_rounded_rect(mem_dc, &r_btn_lock, COLOR_CARD, if s.click_through { COLOR_ACCENT } else { COLOR_BORDER });
            SelectObject(mem_dc, HGDIOBJ(font_body.0));
            SetTextColor(mem_dc, if s.click_through { COLOR_ACCENT } else { COLOR_TEXT_PRI });
            let lock_txt = if s.click_through { "🔒 Locked (Click-Through)" } else { "🔓 Unlocked (Draggable)" };
            let mut r_lock_lbl = r_btn_lock;
            draw_text(mem_dc, lock_txt, &mut r_lock_lbl, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Toggle 2: 24-Hour Format
            let r_btn_fmt = RECT { left: 280, top: act_y + 28, right: 516, bottom: act_y + 66 };
            draw_rounded_rect(mem_dc, &r_btn_fmt, COLOR_CARD, if s.hour24 { COLOR_ACCENT } else { COLOR_BORDER });
            SetTextColor(mem_dc, if s.hour24 { COLOR_ACCENT } else { COLOR_TEXT_PRI });
            let fmt_txt = if s.hour24 { "Clock: 24-Hour" } else { "Clock: 12-Hour" };
            let mut r_fmt_lbl = r_btn_fmt;
            draw_text(mem_dc, fmt_txt, &mut r_fmt_lbl, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Toggle 3: 3D Depth Occlusion
            let r_btn_depth = RECT { left: 24, top: act_y + 74, right: 260, bottom: act_y + 112 };
            draw_rounded_rect(mem_dc, &r_btn_depth, COLOR_CARD, if s.depth_enabled { COLOR_ACCENT } else { COLOR_BORDER });
            SetTextColor(mem_dc, if s.depth_enabled { COLOR_ACCENT } else { COLOR_TEXT_PRI });
            let depth_txt = if s.depth_enabled { "✨ Depth Occlusion: ON" } else { "✨ Depth Occlusion: OFF" };
            let mut r_depth_lbl = r_btn_depth;
            draw_text(mem_dc, depth_txt, &mut r_depth_lbl, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Toggle 4: Show Date Row
            let r_btn_date = RECT { left: 280, top: act_y + 74, right: 516, bottom: act_y + 112 };
            draw_rounded_rect(mem_dc, &r_btn_date, COLOR_CARD, if s.show_date { COLOR_ACCENT } else { COLOR_BORDER });
            SetTextColor(mem_dc, if s.show_date { COLOR_ACCENT } else { COLOR_TEXT_PRI });
            let date_txt = if s.show_date { "📅 Date Row: Shown" } else { "📅 Date Row: Hidden" };
            let mut r_date_lbl = r_btn_date;
            draw_text(mem_dc, date_txt, &mut r_date_lbl, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Reset Position button:
            let r_btn_reset = RECT { left: 24, top: act_y + 120, right: 260, bottom: act_y + 158 };
            draw_rounded_rect(mem_dc, &r_btn_reset, COLOR_CARD, COLOR_BORDER);
            SetTextColor(mem_dc, COLOR_TEXT_PRI);
            let mut r_reset_lbl = r_btn_reset;
            draw_text(mem_dc, "🔄 Reset to Center", &mut r_reset_lbl, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Start with Windows toggle:
            let is_start = start_with_windows();
            let r_btn_start = RECT { left: 280, top: act_y + 120, right: 516, bottom: act_y + 158 };
            draw_rounded_rect(mem_dc, &r_btn_start, COLOR_CARD, if is_start { COLOR_AMBER } else { COLOR_BORDER });
            SetTextColor(mem_dc, if is_start { COLOR_AMBER } else { COLOR_TEXT_PRI });
            let start_txt = if is_start { "🚀 Start with Windows: ON" } else { "🚀 Start with Windows: OFF" };
            let mut r_start_lbl = r_btn_start;
            draw_text(mem_dc, start_txt, &mut r_start_lbl, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // FOOTER:
            SelectObject(mem_dc, HGDIOBJ(font_caption.0));
            SetTextColor(mem_dc, COLOR_TEXT_DIM);
            let mut r_footer = RECT { left: 24, top: 676, right: 516, bottom: 700 };
            draw_text(mem_dc, "ClockScreen v1.0.0 • Pure Native Windows • 0% Idle CPU • Click-through", &mut r_footer, DT_CENTER | DT_SINGLELINE);

            // Blit to screen DC:
            let _ = BitBlt(screen_dc, 0, 0, w, h, Some(mem_dc), 0, 0, SRCCOPY);

            // Cleanup:
            SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(HGDIOBJ(h_bmp.0));
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(Some(hwnd), screen_dc);

            let _ = DeleteObject(HGDIOBJ(font_title.0));
            let _ = DeleteObject(HGDIOBJ(font_subhead.0));
            let _ = DeleteObject(HGDIOBJ(font_body.0));
            let _ = DeleteObject(HGDIOBJ(font_caption.0));

            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xffff) as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i32;

            let mut s = settings::load();
            let mut changed = false;

            // Check Face Cards:
            for i in 0..6 {
                let col = i % 2;
                let row = i / 2;
                let card_x = 24 + col * 252;
                let card_y = 110 + row * 62;
                if x >= card_x && x <= card_x + 240 && y >= card_y && y <= card_y + 54 {
                    s.face = Face::ALL[i as usize];
                    changed = true;
                    break;
                }
            }

            // Check Tint Pills:
            let tint_y = 310;
            for i in 0..5 {
                let pill_x = 24 + i * 98;
                if x >= pill_x && x <= pill_x + 90 && y >= tint_y + 28 && y <= tint_y + 56 {
                    s.tint = Tint::ALL[i as usize];
                    changed = true;
                    break;
                }
            }

            // Check Scale Slider click:
            let slider_y = 388;
            if x >= 24 && x <= 516 && y >= slider_y + 16 && y <= slider_y + 40 {
                let pct = ((x - 24) as f32 / (516.0 - 24.0)).clamp(0.0, 1.0);
                s.scale = SCALE_MIN + pct * (SCALE_MAX - SCALE_MIN);
                is_dragging_slider().store(true, Ordering::Relaxed);
                changed = true;
            }

            // Check Opacity Slider click:
            let op_y = slider_y + 46;
            if x >= 24 && x <= 516 && y >= op_y + 16 && y <= op_y + 40 {
                let pct = ((x - 24) as f32 / (516.0 - 24.0)).clamp(0.0, 1.0);
                s.opacity = OPACITY_MIN + pct * (OPACITY_MAX - OPACITY_MIN);
                is_dragging_slider().store(true, Ordering::Relaxed);
                changed = true;
            }

            // Check Action Buttons:
            let act_y = 480;
            // Lock toggle:
            if x >= 24 && x <= 260 && y >= act_y + 28 && y <= act_y + 66 {
                s.click_through = !s.click_through;
                changed = true;
            }
            // 24h toggle:
            if x >= 280 && x <= 516 && y >= act_y + 28 && y <= act_y + 66 {
                s.hour24 = !s.hour24;
                changed = true;
            }
            // Depth toggle:
            if x >= 24 && x <= 260 && y >= act_y + 74 && y <= act_y + 112 {
                s.depth_enabled = !s.depth_enabled;
                changed = true;
            }
            // Date toggle:
            if x >= 280 && x <= 516 && y >= act_y + 74 && y <= act_y + 112 {
                s.show_date = !s.show_date;
                changed = true;
            }
            // Reset position:
            if x >= 24 && x <= 260 && y >= act_y + 120 && y <= act_y + 158 {
                request_clock_reset();
            }
            // Start with Windows toggle:
            if x >= 280 && x <= 516 && y >= act_y + 120 && y <= act_y + 158 {
                let current = start_with_windows();
                let _ = set_start_with_windows(!current);
                InvalidateRect(Some(hwnd), None, false);
            }

            if changed {
                settings::save(s);
                request_clock_reload();
                InvalidateRect(Some(hwnd), None, false);
            }

            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if is_dragging_slider().load(Ordering::Relaxed) && (wparam.0 & 0x0001 /* MK_LBUTTON */) != 0 {
                let x = (lparam.0 & 0xffff) as i32;
                let y = ((lparam.0 >> 16) & 0xffff) as i32;

                let mut s = settings::load();
                let slider_y = 388;
                let op_y = slider_y + 46;

                if y < op_y {
                    let pct = ((x - 24) as f32 / (516.0 - 24.0)).clamp(0.0, 1.0);
                    s.scale = SCALE_MIN + pct * (SCALE_MAX - SCALE_MIN);
                } else {
                    let pct = ((x - 24) as f32 / (516.0 - 24.0)).clamp(0.0, 1.0);
                    s.opacity = OPACITY_MIN + pct * (OPACITY_MAX - OPACITY_MIN);
                }

                settings::save(s);
                request_clock_reload();
                InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            is_dragging_slider().store(false, Ordering::Relaxed);
            LRESULT(0)
        }
        WM_CLOSE => {
            // Hide to tray on close, keeping the clock running:
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_DESTROY => {
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
