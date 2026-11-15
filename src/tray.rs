//! Windows System Tray Integration & Quick Action Context Menu.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW,
    PostQuitMessage, SetForegroundWindow, TrackPopupMenu, IDI_APPLICATION,
    MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WM_USER,
};

use crate::clock_window::{request_clock_reload, request_clock_reset};
use crate::settings::{self, set_start_with_windows, start_with_windows};
use crate::studio_window::show_studio_window;

pub const WM_TRAYICON: u32 = WM_USER + 201;

pub const ID_SETTINGS: usize = 1001;
pub const ID_LOCK: usize = 1002;
pub const ID_NEXT_FACE: usize = 1003;
pub const ID_NEXT_TINT: usize = 1004;
pub const ID_RESET: usize = 1005;
pub const ID_STARTUP: usize = 1006;
pub const ID_QUIT: usize = 1007;

pub unsafe fn add_tray_icon(hwnd: HWND) {
    let h_icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: h_icon,
        ..Default::default()
    };

    let tip = w!("ClockScreen — Desktop Wallpaper Depth Clock");
    let tip_slice = tip.as_wide();
    let len = tip_slice.len().min(nid.szTip.len() - 1);
    nid.szTip[..len].copy_from_slice(&tip_slice[..len]);

    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

pub unsafe fn remove_tray_icon(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}

pub unsafe fn show_tray_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };

    let s = settings::load();

    let _ = AppendMenuW(menu, MF_STRING, ID_SETTINGS, w!("ClockScreen Studio (Settings)..."));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

    let lock_flags = if s.click_through { MF_CHECKED } else { MF_UNCHECKED };
    let _ = AppendMenuW(menu, MF_STRING | lock_flags, ID_LOCK, w!("Lock Position (Click-Through)"));

    let _ = AppendMenuW(menu, MF_STRING, ID_NEXT_FACE, w!("Next Clock Face"));
    let _ = AppendMenuW(menu, MF_STRING, ID_NEXT_TINT, w!("Next Tint Style"));
    let _ = AppendMenuW(menu, MF_STRING, ID_RESET, w!("Reset to Center"));

    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

    let startup_flags = if start_with_windows() { MF_CHECKED } else { MF_UNCHECKED };
    let _ = AppendMenuW(menu, MF_STRING | startup_flags, ID_STARTUP, w!("Start with Windows"));

    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quit ClockScreen"));

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);

    let _ = SetForegroundWindow(hwnd);
    let res = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
        pt.x,
        pt.y,
        None,
        hwnd,
        None,
    );

    let _ = DestroyMenu(menu);

    if res.0 > 0 {
        handle_tray_command(hwnd, res.0 as usize);
    }
}

pub unsafe fn handle_tray_command(hwnd: HWND, command_id: usize) {
    match command_id {
        ID_SETTINGS => {
            show_studio_window();
        }
        ID_LOCK => {
            let mut s = settings::load();
            s.click_through = !s.click_through;
            settings::save(s);
            request_clock_reload();
        }
        ID_NEXT_FACE => {
            let mut s = settings::load();
            s.face = s.face.next();
            settings::save(s);
            request_clock_reload();
        }
        ID_NEXT_TINT => {
            let mut s = settings::load();
            s.tint = s.tint.next();
            settings::save(s);
            request_clock_reload();
        }
        ID_RESET => {
            request_clock_reset();
        }
        ID_STARTUP => {
            let current = start_with_windows();
            let _ = set_start_with_windows(!current);
        }
        ID_QUIT => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
        }
        _ => {}
    }
}
