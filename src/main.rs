//! ClockScreen: The Native Wallpaper Depth-Effect Clock & Studio App for Windows.
//!
//! Features:
//! - True Win32 UpdateLayeredWindow with pre-multiplied alpha: 0% CPU, 0% GPU, zero white boxes.
//! - Wallpaper Depth Occlusion: sandwiches clock numbers behind wallpaper foreground subjects.
//! - ClockScreen Studio: Apple/macOS Sonoma dark-glass management GUI.
//! - System Tray integration with quick controls.
//! - Drag-to-position with smooth mouse tracking, and toggleable desktop click-through.
//! - Anchored to desktop z-order (HWND_BOTTOM) and immune to Win+D minimization.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod clock_window;
mod depth;
mod faces;
mod glyphs;
mod settings;
mod studio_window;
mod tray;

use std::sync::atomic::{AtomicU32, Ordering};
use windows::core::w;
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, MSG};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, FindWindowW, GetMessageW, KillTimer, PostQuitMessage,
    SetForegroundWindow, SetTimer, TranslateMessage, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK,
    WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER,
};

static LAST_MINUTE: AtomicU32 = AtomicU32::new(u32::MAX);

fn current_minute() -> u32 {
    let mut now = windows::Win32::System::SystemInformation::SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut now) };
    (now.wHour as u32) * 60 + (now.wMinute as u32)
}

fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("clockscreen {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    unsafe {
        // High DPI awareness:
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // Single instance check:
        let mutex_name = w!("Global\\ClockScreenAppMutex");
        let mutex = CreateMutexW(None, true, mutex_name);
        if windows::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
            // Already running: bring Studio to foreground:
            if let Ok(existing) = FindWindowW(w!("ClockScreenStudioClass"), windows::core::PCWSTR::null()) {
                let _ = SetForegroundWindow(existing);
            }
            return;
        }

        let saved = settings::load();

        // Load custom or default depth mask if configured:
        if saved.depth_enabled {
            if let Some(path) = &saved.mask_path {
                if let Some(mask) = depth::DepthMask::load_from_png(path) {
                    clock_window::set_depth_mask(Some(mask));
                }
            } else if std::path::Path::new("assets/mask.png").exists() {
                if let Some(mask) = depth::DepthMask::load_from_png("assets/mask.png") {
                    clock_window::set_depth_mask(Some(mask));
                }
            }
        }

        // Create the desktop layered clock overlay:
        let clock_hwnd = match clock_window::create_clock_overlay(&saved) {
            Ok(hwnd) => hwnd,
            Err(e) => {
                eprintln!("clockscreen: could not create desktop clock window: {e}");
                return;
            }
        };

        // Create System Tray icon:
        tray::add_tray_icon(clock_hwnd);

        // If user launched with `--settings`, open Studio immediately:
        if std::env::args().any(|arg| arg == "--settings") {
            studio_window::show_studio_window();
        }

        LAST_MINUTE.store(current_minute(), Ordering::Relaxed);

        // Timer for top of the minute updates:
        let timer_id = SetTimer(Some(clock_hwnd), 1, 1000, None);

        // Standard Win32 Message Loop:
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            match msg.message {
                WM_TIMER => {
                    let min = current_minute();
                    if min != LAST_MINUTE.swap(min, Ordering::Relaxed) {
                        let current_settings = settings::load();
                        clock_window::render_clock(clock_hwnd, &current_settings);
                    }
                }
                tray::WM_TRAYICON => {
                    let event = (msg.lParam.0 & 0xffff) as u32;
                    match event {
                        WM_RBUTTONUP => {
                            tray::show_tray_menu(clock_hwnd);
                        }
                        WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                            studio_window::show_studio_window();
                        }
                        _ => {}
                    }
                }
                WM_COMMAND => {
                    let command_id = (msg.wParam.0 & 0xffff) as usize;
                    tray::handle_tray_command(clock_hwnd, command_id);
                }
                WM_DESTROY => {
                    tray::remove_tray_icon(clock_hwnd);
                    let _ = KillTimer(Some(clock_hwnd), timer_id);
                    PostQuitMessage(0);
                }
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        let _ = KillTimer(Some(clock_hwnd), timer_id);
        tray::remove_tray_icon(clock_hwnd);
        if let Ok(m) = mutex {
            let _ = windows::Win32::Foundation::CloseHandle(m);
        }
    }
}
