//! The Win32 pieces Rune does not expose.
//!
//! A click-through window receives no mouse input at all, so hold-to-drag and Ctrl+wheel are
//! driven by a low-level mouse hook (`WH_MOUSE_LL`) on its own thread: the hook only watches,
//! and moves the window with `SetWindowPos`, so clicks still reach the desktop.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SYSTEMTIME, WPARAM};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, GWL_EXSTYLE, HWND_BOTTOM, MSG, MSLLHOOKSTRUCT,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

use crate::settings::{self, SCALE_MAX, SCALE_MIN};

/// How long the left button must be held before the clock starts following the pointer.
const HOLD_MS: u64 = 500;

/// The window's own copy of the shared state the input thread writes and the view reads.
pub struct Input {
    scale: AtomicU32,
    reset: AtomicBool,
    reload: AtomicBool,
    panel: AtomicBool,
}

impl Input {
    pub fn new(scale: f32) -> Self {
        Self {
            scale: AtomicU32::new(scale.to_bits()),
            reset: AtomicBool::new(false),
            reload: AtomicBool::new(false),
            panel: AtomicBool::new(false),
        }
    }
    pub fn scale(&self) -> f32 {
        f32::from_bits(self.scale.load(Ordering::Relaxed))
    }
    fn set_scale(&self, value: f32) {
        self.scale.store(value.to_bits(), Ordering::Relaxed);
    }
    pub fn request_reset(&self) {
        self.reset.store(true, Ordering::Relaxed);
    }
    pub fn take_reset(&self) -> bool {
        self.reset.swap(false, Ordering::Relaxed)
    }
    /// Ask the clock to re-read `settings.json` (face, tint, opacity, clock format).
    pub fn request_reload(&self) {
        self.reload.store(true, Ordering::Relaxed);
    }
    pub fn take_reload(&self) -> bool {
        self.reload.swap(false, Ordering::Relaxed)
    }
    /// Ask the settings panel to re-read `settings.json` (the tray changed the face).
    pub fn request_panel(&self) {
        self.panel.store(true, Ordering::Relaxed);
    }
    pub fn take_panel(&self) -> bool {
        self.panel.swap(false, Ordering::Relaxed)
    }
}

struct Hook {
    /// `HWND` is a bare pointer and so is neither `Send` nor `Sync`; the address is.
    hwnd: isize,
    input: Arc<Input>,
}

static HOOK: OnceLock<Hook> = OnceLock::new();

static PRESSED: AtomicBool = AtomicBool::new(false);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static PRESS_X: AtomicU32 = AtomicU32::new(0);
static PRESS_Y: AtomicU32 = AtomicU32::new(0);
static WIN_X: AtomicU32 = AtomicU32::new(0);
static WIN_Y: AtomicU32 = AtomicU32::new(0);
static PRESS_AT: AtomicU32 = AtomicU32::new(0);

fn millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn rect_of(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect).ok()? };
    Some(rect)
}

fn pointer() -> Option<POINT> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).ok()? };
    Some(point)
}

fn inside(rect: &RECT, point: POINT) -> bool {
    point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        if let Some(hook) = HOOK.get() {
            let hwnd = HWND(hook.hwnd as *mut core::ffi::c_void);
            match wparam.0 as u32 {
                WM_LBUTTONDOWN => {
                    if let (Some(point), Some(rect)) = (pointer(), rect_of(hwnd)) {
                        if inside(&rect, point) {
                            PRESS_X.store(point.x as u32, Ordering::Relaxed);
                            PRESS_Y.store(point.y as u32, Ordering::Relaxed);
                            WIN_X.store(rect.left as u32, Ordering::Relaxed);
                            WIN_Y.store(rect.top as u32, Ordering::Relaxed);
                            PRESS_AT.store(millis() as u32, Ordering::Relaxed);
                            DRAGGING.store(false, Ordering::Relaxed);
                            PRESSED.store(true, Ordering::Relaxed);
                        }
                    }
                }
                WM_MOUSEMOVE => {
                    if PRESSED.load(Ordering::Relaxed) {
                        let held = (millis() as u32).wrapping_sub(PRESS_AT.load(Ordering::Relaxed)) as u64;
                        if !DRAGGING.load(Ordering::Relaxed) && held >= HOLD_MS {
                            DRAGGING.store(true, Ordering::Relaxed);
                        }
                        if DRAGGING.load(Ordering::Relaxed) {
                            if let Some(point) = pointer() {
                                let dx = point.x - PRESS_X.load(Ordering::Relaxed) as i32;
                                let dy = point.y - PRESS_Y.load(Ordering::Relaxed) as i32;
                                let x = WIN_X.load(Ordering::Relaxed) as i32 + dx;
                                let y = WIN_Y.load(Ordering::Relaxed) as i32 + dy;
                                let _ = SetWindowPos(
                                    hwnd,
                                    None,
                                    x,
                                    y,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_NOACTIVATE,
                                );
                            }
                        }
                    }
                }
                WM_LBUTTONUP => {
                    let was_dragging = DRAGGING.swap(false, Ordering::Relaxed);
                    PRESSED.store(false, Ordering::Relaxed);
                    if was_dragging {
                        if let Some(rect) = rect_of(hwnd) {
                            // Re-read the file and change only the position: the settings window
                            // may have written a new face or opacity since this process started.
                            let mut saved = settings::load();
                            saved.x = Some(rect.left);
                            saved.y = Some(rect.top);
                            settings::save(saved);
                        }
                    }
                }
                WM_MOUSEWHEEL => {
                    let ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
                    if ctrl {
                        if let (Some(point), Some(rect)) = (pointer(), rect_of(hwnd)) {
                            if inside(&rect, point) {
                                let wheel = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                                let delta = ((wheel.mouseData >> 16) as u16) as i16 as i32;
                                if delta != 0 {
                                    let step = if delta > 0 { 0.1 } else { -0.1 };
                                    let next =
                                        (hook.input.scale() + step).clamp(SCALE_MIN, SCALE_MAX);
                                    hook.input.set_scale(next);
                                    let mut saved = settings::load();
                                    saved.scale = next;
                                    settings::save(saved);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Watches the mouse from a thread of its own: a low-level hook needs a message loop, and
/// `run_windows` already owns the main one.
pub fn watch_mouse(hwnd: HWND, input: Arc<Input>) {
    let _ = HOOK.set(Hook { hwnd: hwnd.0 as isize, input });
    std::thread::spawn(|| unsafe {
        let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), None, 0) {
            Ok(hook) => hook,
            Err(error) => {
                eprintln!("clockscreen: mouse hook unavailable: {error}");
                return;
            }
        };
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = UnhookWindowsHookEx(hook);
    });
}

/// The native window behind a Rune handle, for the Win32 calls Rune doesn't wrap.
pub fn hwnd_of(window: &rune::WindowHandle) -> Option<HWND> {
    window
        .with_raw_handle(|handle| match handle.as_raw() {
            raw_window_handle::RawWindowHandle::Win32(win32) => {
                Some(HWND(win32.hwnd.get() as _))
            }
            _ => None,
        })
        .ok()
        .flatten()
}

/// Bottom of the z-order, no taskbar button, no Alt-Tab entry, never activated.
pub fn pin_to_desktop(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = (style | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize)
            & !(WS_EX_APPWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub fn screen_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

pub struct Now {
    pub hour: u32,
    pub minute: u32,
    pub weekday: u32,
    pub day: u32,
    pub month: u32,
}

pub fn now() -> Now {
    let time: SYSTEMTIME = unsafe { GetLocalTime() };
    Now {
        hour: time.wHour as u32,
        minute: time.wMinute as u32,
        weekday: time.wDayOfWeek as u32,
        day: time.wDay as u32,
        month: time.wMonth as u32,
    }
}

// The Run key is written through `reg.exe`: one process on a rare toggle, and no signature
// guesswork against the `windows` crate's registry bindings.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "clockscreen";

pub fn start_with_windows() -> bool {
    std::process::Command::new("reg")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn set_start_with_windows(enabled: bool) -> bool {
    let mut command = std::process::Command::new("reg");
    if enabled {
        let exe = std::env::current_exe().unwrap_or_default();
        let target = format!("\"{}\"", exe.display());
        command.args([
            "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", target.as_str(), "/f",
        ]);
    } else {
        // A missing value is already "off", so a failed delete still counts.
        command.args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    }
    let wrote = command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if enabled { wrote } else { wrote || !start_with_windows() }
}
