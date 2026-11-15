//! The Native Windows Desktop Layered Overlay Window.
//! Powered by Win32 UpdateLayeredWindow with pre-multiplied alpha:
//! 0% CPU, 0% GPU, true per-pixel transparency, and zero white boxes.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, SYSTEMTIME, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HDC, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    PostMessageW, RegisterClassW, SendMessageW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, HTCAPTION, HWND_BOTTOM, SM_CXSCREEN, SM_CYSCREEN, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, ULW_ALPHA, WINDOWPOS, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN, WM_USER,
    WM_WINDOWPOSCHANGING, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::depth::{apply_depth_occlusion, DepthMask};
use crate::faces::style_for_face;
use crate::glyphs::render_clock_layout;
use crate::settings::{self, Settings, SCALE_MAX, SCALE_MIN};

pub const WM_CLOCK_RELOAD: u32 = WM_USER + 101;
pub const WM_CLOCK_RESET: u32 = WM_USER + 102;

static CLOCK_HWND: AtomicIsize = AtomicIsize::new(0);
static DEPTH_MASK: Mutex<Option<Arc<DepthMask>>> = Mutex::new(None);
static SYSTEM_DARK: AtomicBool = AtomicBool::new(true);

pub fn set_clock_hwnd(hwnd: HWND) {
    CLOCK_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
}

pub fn get_clock_hwnd() -> Option<HWND> {
    let raw = CLOCK_HWND.load(Ordering::Relaxed);
    if raw == 0 {
        None
    } else {
        Some(HWND(raw as *mut core::ffi::c_void))
    }
}

pub fn request_clock_reload() {
    if let Some(hwnd) = get_clock_hwnd() {
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOCK_RELOAD, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn request_clock_reset() {
    if let Some(hwnd) = get_clock_hwnd() {
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOCK_RESET, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn set_depth_mask(mask: Option<DepthMask>) {
    if let Ok(mut held) = DEPTH_MASK.lock() {
        *held = mask.map(Arc::new);
    }
    request_clock_reload();
}

const WEEKDAYS: [&str; 7] = [
    "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
];
const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

fn format_clock(now: &SYSTEMTIME, hour24: bool) -> (String, String) {
    let time_str = if hour24 {
        format!("{:02}:{:02}", now.wHour, now.wMinute)
    } else {
        let hour = now.wHour % 12;
        let hour = if hour == 0 { 12 } else { hour };
        format!("{:02}:{:02}", hour, now.wMinute)
    };

    let weekday = WEEKDAYS.get(now.wDayOfWeek as usize).copied().unwrap_or("");
    let month = MONTHS.get(now.wMonth.saturating_sub(1) as usize).copied().unwrap_or("");

    let date_str = if hour24 {
        format!("{weekday} {} {month}", now.wDay)
    } else {
        let meridiem = if now.wHour < 12 { "AM" } else { "PM" };
        format!("{weekday} {} {month}  ·  {meridiem}", now.wDay)
    };

    (time_str, date_str)
}

pub unsafe fn create_clock_overlay(settings: &Settings) -> windows::core::Result<HWND> {
    let instance = GetModuleHandleW(None)?;
    let class_name = w!("ClockScreenOverlayClass");

    let wc = WNDCLASSW {
        lpfnWndProc: Some(clock_wnd_proc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    let _ = RegisterClassW(&wc);

    let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW;
    if settings.click_through {
        ex_style |= WS_EX_TRANSPARENT;
    }

    let screen_w = GetSystemMetrics(SM_CXSCREEN);
    let screen_h = GetSystemMetrics(SM_CYSCREEN);

    let initial_x = settings.x.unwrap_or((screen_w - 600) / 2);
    let initial_y = settings.y.unwrap_or((screen_h - 300) / 2);

    let hwnd = CreateWindowExW(
        ex_style,
        class_name,
        w!("ClockScreen Overlay"),
        WS_POPUP,
        initial_x,
        initial_y,
        600,
        300,
        None,
        None,
        Some(instance.into()),
        None,
    )?;

    set_clock_hwnd(hwnd);

    // Pin to desktop bottom:
    let _ = SetWindowPos(
        hwnd,
        HWND_BOTTOM,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );

    // Initial render:
    render_clock(hwnd, settings);

    let _ = ShowWindow(hwnd, SW_SHOW);
    Ok(hwnd)
}

pub unsafe fn update_click_through(hwnd: HWND, click_through: bool) {
    let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if click_through {
        ex_style |= WS_EX_TRANSPARENT.0 as isize;
    } else {
        ex_style &= !(WS_EX_TRANSPARENT.0 as isize);
    }
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
    let _ = SetWindowPos(
        hwnd,
        HWND::default(),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
}

pub unsafe fn render_clock(hwnd: HWND, settings: &Settings) {
    let mut now = SYSTEMTIME::default();
    GetLocalTime(&mut now);

    let (time_str, date_str) = format_clock(&now, settings.hour24);
    let style = style_for_face(
        settings.face,
        settings.tint,
        SYSTEM_DARK.load(Ordering::Relaxed),
        settings.opacity,
    );

    let Some(mut raster) = render_clock_layout(
        &time_str,
        &date_str,
        &style,
        settings.scale,
        settings.show_date,
    ) else {
        return;
    };

    // Apply depth occlusion if enabled:
    if settings.depth_enabled {
        if let Ok(held) = DEPTH_MASK.lock() {
            if let Some(mask) = held.as_ref() {
                apply_depth_occlusion(&mut raster, mask, 0, 0);
            }
        }
    }

    let width = raster.width as i32;
    let height = raster.height as i32;

    let screen_dc = GetDC(None);
    let mem_dc = CreateCompatibleDC(Some(screen_dc));
    if mem_dc.0.is_null() {
        let _ = ReleaseDC(None, screen_dc);
        return;
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // Top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut ppv_bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let Ok(h_bitmap) = CreateDIBSection(
        Some(screen_dc),
        Some(&bmi),
        DIB_RGB_COLORS,
        &mut ppv_bits,
        HANDLE::default(),
        0,
    ) else {
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        return;
    };

    if !ppv_bits.is_null() {
        std::ptr::copy_nonoverlapping(raster.bgra.as_ptr(), ppv_bits as *mut u8, raster.bgra.len());
    }

    let old_bmp = SelectObject(mem_dc, HGDIOBJ(h_bitmap.0));

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    let pt_dst = POINT { x: rect.left, y: rect.top };
    let size = SIZE { cx: width, cy: height };
    let pt_src = POINT { x: 0, y: 0 };

    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    let _ = windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow(
        hwnd,
        Some(screen_dc),
        Some(&pt_dst),
        Some(&size),
        Some(mem_dc),
        Some(&pt_src),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );

    SelectObject(mem_dc, old_bmp);
    let _ = DeleteObject(HGDIOBJ(h_bitmap.0));
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(None, screen_dc);
}

pub unsafe extern "system" fn clock_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            // Drag the clock smoothly when unlocked:
            let _ = SendMessageW(
                hwnd,
                WM_NCLBUTTONDOWN,
                Some(WPARAM(HTCAPTION as usize)),
                Some(lparam),
            );
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let mut current = settings::load();
                current.x = Some(rect.left);
                current.y = Some(rect.top);
                settings::save(current);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let ctrl = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            if ctrl {
                let delta = ((wparam.0 >> 16) as u16) as i16;
                let step = if delta > 0 { 0.05 } else { -0.05 };
                let mut current = settings::load();
                current.scale = (current.scale + step).clamp(SCALE_MIN, SCALE_MAX);
                settings::save(current.clone());
                render_clock(hwnd, &current);
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGING => {
            let pos = &mut *(lparam.0 as *mut WINDOWPOS);
            // Prevent Windows from hiding or minimizing clock on Win+D (Show Desktop):
            if pos.x == -32000 || (pos.flags.0 & 0x0080 /* SWP_HIDEWINDOW */) != 0 {
                pos.flags.0 |= 0x0001 | 0x0002; // SWP_NOSIZE | SWP_NOMOVE
                pos.flags.0 &= !0x0080;         // Clear SWP_HIDEWINDOW
            }
            pos.hwndInsertAfter = HWND_BOTTOM;
            pos.flags.0 &= !0x0004; // Clear SWP_NOZORDER
            LRESULT(0)
        }
        WM_CLOCK_RELOAD => {
            let current = settings::load();
            update_click_through(hwnd, current.click_through);
            render_clock(hwnd, &current);
            LRESULT(0)
        }
        WM_CLOCK_RESET => {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            let new_x = (screen_w - w) / 2;
            let new_y = (screen_h - h) / 2;
            let _ = SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                new_x,
                new_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            let mut current = settings::load();
            current.x = Some(new_x);
            current.y = Some(new_y);
            settings::save(current.clone());
            render_clock(hwnd, &current);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            let current = settings::load();
            render_clock(hwnd, &current);
            LRESULT(0)
        }
        WM_DESTROY => {
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
