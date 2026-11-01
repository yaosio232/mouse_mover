#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use azu_sanpo::{
    build_azki_path, cancel_worker, center_path, countdown_seconds, is_physical_keyboard,
    is_physical_mouse, replace_worker_cancel, ActivityState, Bounds, Status, WorkerCancel,
};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicIsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, SetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        CreateEllipticRgn, CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush,
        DeleteObject, DrawFocusRect, DrawTextW, FillRect, FillRgn, FrameRgn, GradientFill,
        InvalidateRect, Polyline, RedrawWindow, RestoreDC, SaveDC, SelectClipRgn, SelectObject,
        SetBkColor, SetBkMode, SetTextColor, UpdateWindow, DT_CENTER, DT_SINGLELINE, DT_VCENTER,
        FW_NORMAL, FW_SEMIBOLD, GRADIENT_FILL_RECT_V, GRADIENT_RECT, HBRUSH, HDC, HFONT, PS_SOLID,
        RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, TRANSPARENT, TRIVERTEX,
    },
    System::LibraryLoader::GetModuleHandleW,
    System::SystemInformation::GetTickCount,
    System::SystemServices::SS_OWNERDRAW,
    UI::{
        Controls::{
            InitCommonControls, BST_CHECKED, CDDS_PREPAINT, CDIS_DISABLED, CDIS_FOCUS, CDIS_HOT,
            CDIS_SELECTED, CDRF_SKIPDEFAULT, DRAWITEMSTRUCT, NMCUSTOMDRAW, NM_CUSTOMDRAW,
            ODS_FOCUS, ODS_SELECTED, UDM_SETBUDDY, UDM_SETPOS32, UDM_SETRANGE32, UDS_ALIGNRIGHT,
            UDS_ARROWKEYS, UDS_SETBUDDYINT,
        },
        HiDpi::GetDpiForSystem,
        Input::KeyboardAndMouse::{
            GetLastInputInfo, RegisterHotKey, SendInput, UnregisterHotKey, INPUT, INPUT_0,
            INPUT_MOUSE, LASTINPUTINFO, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
            VK_F10,
        },
        WindowsAndMessaging::*,
    },
};

const WM_INPUT_EVENT: u32 = WM_APP + 1;
const WM_TIMER_ID: usize = 1;
const WM_HOTKEY_ID: i32 = 1;
const TRANSPARENCY_KEY: u32 = 1 | (1 << 8) | (1 << 16);
static WINDOW: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static KEY_HOOK: AtomicIsize = AtomicIsize::new(0);
static WORKER_CANCEL: WorkerCancel = Mutex::new(None);
static mut INSTANCE: isize = 0;
static mut DPI: i32 = 96;
static mut TIMER_RUNNING: bool = false;
static mut BACKGROUND_TRANSPARENT: bool = false;

#[derive(Clone, Copy)]
struct Ui {
    hwnd: HWND,
    button: HWND,
    background_toggle: HWND,
    lamp: HWND,
    status_badge: HWND,
    idle_badge: HWND,
    in_time: HWND,
    check: HWND,
    hours: HWND,
    minutes: HWND,
    seconds: HWND,
}
static mut UI: Option<Ui> = None;
static mut LAMP_BRUSHES: [HBRUSH; 3] = [std::ptr::null_mut(); 3];
static mut STATUS_BRUSHES: [HBRUSH; 3] = [std::ptr::null_mut(); 3];
static mut BG_BRUSH: HBRUSH = std::ptr::null_mut();
static mut KEY_BRUSH: HBRUSH = std::ptr::null_mut();
static mut SURFACE_BRUSH: HBRUSH = std::ptr::null_mut();
static mut BUTTON_BORDER_BRUSH: HBRUSH = std::ptr::null_mut();
static mut CHECK_BRUSH: HBRUSH = std::ptr::null_mut();
static mut CHECK_HOT_BRUSH: HBRUSH = std::ptr::null_mut();
static mut IDLE_BORDER_BRUSH: HBRUSH = std::ptr::null_mut();
static mut FONTS: [HFONT; 6] = [std::ptr::null_mut(); 6];
#[cfg(test)]
static TEST_UI_INVALIDATIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
thread_local! { static STATE: RefCell<ActivityState> = RefCell::new(ActivityState::new(Instant::now())); }

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            unsafe {
                if !init_theme_resources() || !create_controls(hwnd) {
                    message("Unable to create the application controls.");
                    return -1;
                }
                RegisterHotKey(hwnd, WM_HOTKEY_ID, 0, VK_F10 as u32);
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                update_display(Some(Duration::ZERO));
            }
            0
        }
        WM_COMMAND => {
            let id = (wp & 0xffff) as i32;
            let notification = ((wp >> 16) & 0xffff) as u16;
            unsafe {
                if id == 100 {
                    toggle();
                } else if id == 117 {
                    toggle_background();
                } else if id == 103
                    || ((id == 104 || id == 105 || id == 111) && notification == EN_CHANGE as u16)
                {
                    configure_countdown();
                }
            }
            0
        }
        WM_NOTIFY if lp != 0 => {
            let header = unsafe { &*(lp as *const windows_sys::Win32::UI::Controls::NMHDR) };
            if header.idFrom == 103 && header.code == NM_CUSTOMDRAW {
                let draw = unsafe { &*(lp as *const NMCUSTOMDRAW) };
                if draw.dwDrawStage == CDDS_PREPAINT {
                    unsafe { draw_stop_checkbox(draw) };
                    return CDRF_SKIPDEFAULT as LRESULT;
                }
            }
            0
        }
        WM_HOTKEY if wp as i32 == WM_HOTKEY_ID => {
            unsafe {
                toggle();
            }
            0
        }
        WM_INPUT_EVENT => {
            unsafe {
                on_input();
            }
            0
        }
        WM_TIMER if wp == WM_TIMER_ID => {
            unsafe {
                tick();
            }
            0
        }
        WM_ERASEBKGND => {
            unsafe {
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect);
                FillRect(wp as HDC, &rect, background_brush());
            }
            1
        }
        WM_DRAWITEM => {
            if lp != 0 {
                let item = unsafe { &*(lp as *const DRAWITEMSTRUCT) };
                unsafe {
                    if item.CtlID == 100 {
                        draw_primary_button(item);
                        return 1;
                    }
                    if item.CtlID == 101 {
                        draw_status_dot(item);
                        return 1;
                    }
                    if item.CtlID == 102 {
                        draw_status_badge(item);
                        return 1;
                    }
                    if item.CtlID == 114 {
                        draw_idle_badge(item);
                        return 1;
                    }
                    if matches!(item.CtlID, 108 | 109 | 113 | 116) {
                        draw_floating_static(item);
                        return 1;
                    }
                    if item.CtlID == 117 {
                        draw_transparency_toggle(item);
                        return 1;
                    }
                }
            }
            0
        }
        val if val == WM_APP + 3 => {
            unsafe {
                stop(false);
                let detail = if wp == 2 {
                    "SendInput failed".to_owned()
                } else {
                    format!(
                        "SetCursorPos failed:\r\n{}",
                        std::io::Error::from_raw_os_error(lp as i32)
                    )
                };
                message(&format!("Unexpected error:\r\n{detail}"));
            }
            0
        }
        WM_CTLCOLORSTATIC => unsafe {
            SetTextColor(wp as HDC, rgb(23, 32, 51));
            SetBkMode(wp as HDC, TRANSPARENT as i32);
            background_brush() as LRESULT
        },
        WM_CTLCOLOREDIT => unsafe {
            SetTextColor(wp as HDC, rgb(23, 32, 51));
            SetBkColor(wp as HDC, rgb(255, 255, 255));
            SURFACE_BRUSH as LRESULT
        },
        WM_CTLCOLORBTN => unsafe {
            SetTextColor(wp as HDC, rgb(23, 32, 51));
            SetBkMode(wp as HDC, TRANSPARENT as i32);
            background_brush() as LRESULT
        },
        WM_CLOSE => {
            unsafe {
                stop(false);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                UnregisterHotKey(hwnd, WM_HOTKEY_ID);
                KillTimer(hwnd, WM_TIMER_ID);
                destroy_hooks();
                PostQuitMessage(0);
            }
            0
        }
        WM_NCDESTROY => {
            let result = unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
            unsafe {
                destroy_theme_resources();
                UI = None;
            }
            result
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

unsafe fn create_controls(hwnd: HWND) -> bool {
    unsafe {
        let make =
            |class: &str, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: isize| {
                let c: Vec<u16> = class.encode_utf16().chain(Some(0)).collect();
                let t: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                let sc = |v| (v * DPI + 48) / 96;
                let control = CreateWindowExW(
                    0,
                    c.as_ptr(),
                    t.as_ptr(),
                    WS_CHILD | WS_VISIBLE | style,
                    sc(x),
                    sc(y),
                    sc(w),
                    sc(h),
                    hwnd,
                    id as _,
                    INSTANCE as _,
                    std::ptr::null(),
                );
                if !control.is_null() {
                    SendMessageW(control, WM_SETFONT, FONTS[0] as usize, 1);
                }
                control
            };
        let button = make(
            "BUTTON",
            "Start (F10)",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            8,
            8,
            292,
            40,
            100,
        );
        let background_toggle = make(
            "BUTTON",
            "Make background transparent",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            306,
            14,
            26,
            28,
            117,
        );
        let lamp = make("STATIC", "", SS_OWNERDRAW, 8, 63, 14, 14, 101);
        let status_badge = make("STATIC", "Not Started", SS_OWNERDRAW, 26, 58, 108, 24, 102);
        let idle_badge = make(
            "STATIC",
            "IDLE 00:00:00",
            SS_OWNERDRAW,
            142,
            58,
            128,
            24,
            114,
        );
        let in_time = make("STATIC", "In 03:00", SS_OWNERDRAW, 276, 58, 56, 24, 116);
        let check = make(
            "BUTTON",
            "Stop after",
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
            8,
            96,
            142,
            26,
            103,
        );
        let hour_label = make("STATIC", "h", SS_OWNERDRAW, 198, 99, 12, 20, 113);
        let min_label = make("STATIC", "m", SS_OWNERDRAW, 262, 99, 12, 20, 108);
        let sec_label_text = make("STATIC", "s", SS_OWNERDRAW, 326, 99, 10, 20, 109);
        let hours = make(
            "EDIT",
            "3",
            WS_BORDER | WS_TABSTOP | ES_NUMBER as u32 | ES_CENTER as u32,
            152,
            96,
            44,
            26,
            111,
        );
        let uphour = make(
            "msctls_updown32",
            "",
            UDS_ALIGNRIGHT | UDS_ARROWKEYS | UDS_SETBUDDYINT,
            0,
            0,
            0,
            0,
            112,
        );
        if [
            button,
            background_toggle,
            lamp,
            status_badge,
            idle_badge,
            in_time,
            check,
            hour_label,
            min_label,
            sec_label_text,
            hours,
            uphour,
        ]
        .iter()
        .any(|control| control.is_null())
        {
            return false;
        }
        SendMessageW(uphour, UDM_SETBUDDY, hours as usize, 0);
        SendMessageW(uphour, UDM_SETRANGE32, 0, 999);
        SendMessageW(uphour, UDM_SETPOS32, 0, 3);
        let minutes = make(
            "EDIT",
            "0",
            WS_BORDER | WS_TABSTOP | ES_NUMBER as u32 | ES_CENTER as u32,
            216,
            96,
            44,
            26,
            104,
        );
        let upmin = make(
            "msctls_updown32",
            "",
            UDS_ALIGNRIGHT | UDS_ARROWKEYS | UDS_SETBUDDYINT,
            0,
            0,
            0,
            0,
            106,
        );
        if minutes.is_null() || upmin.is_null() {
            return false;
        }
        SendMessageW(upmin, UDM_SETBUDDY, minutes as usize, 0);
        SendMessageW(upmin, UDM_SETRANGE32, 0, 59);
        let sec_edit = make(
            "EDIT",
            "0",
            WS_BORDER | WS_TABSTOP | ES_NUMBER as u32 | ES_CENTER as u32,
            280,
            96,
            44,
            26,
            105,
        );
        let upsec = make(
            "msctls_updown32",
            "",
            UDS_ALIGNRIGHT | UDS_ARROWKEYS | UDS_SETBUDDYINT,
            0,
            0,
            0,
            0,
            107,
        );
        if sec_edit.is_null() || upsec.is_null() {
            return false;
        }
        SendMessageW(upsec, UDM_SETBUDDY, sec_edit as usize, 0);
        SendMessageW(upsec, UDM_SETRANGE32, 0, 59);
        SendMessageW(button, WM_SETFONT, FONTS[1] as usize, 1);
        SendMessageW(background_toggle, WM_SETFONT, FONTS[0] as usize, 1);
        SendMessageW(check, WM_SETFONT, FONTS[5] as usize, 1);
        SendMessageW(status_badge, WM_SETFONT, FONTS[3] as usize, 1);
        SendMessageW(idle_badge, WM_SETFONT, FONTS[4] as usize, 1);
        SendMessageW(in_time, WM_SETFONT, FONTS[4] as usize, 1);
        for label in [hour_label, min_label, sec_label_text] {
            SendMessageW(label, WM_SETFONT, FONTS[0] as usize, 1);
        }
        for edit in [hours, minutes, sec_edit] {
            SendMessageW(edit, WM_SETFONT, FONTS[2] as usize, 1);
        }
        UI = Some(Ui {
            hwnd,
            button,
            background_toggle,
            lamp,
            status_badge,
            idle_badge,
            in_time,
            check,
            hours,
            minutes,
            seconds: sec_edit,
        });
    }
    true
}

unsafe fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe fn set_text(hwnd: HWND, text: &str) -> bool {
    let text = unsafe { wide(text) };
    let mut current = vec![0u16; text.len()];
    let length = unsafe { GetWindowTextW(hwnd, current.as_mut_ptr(), current.len() as i32) };
    if current[..length.max(0) as usize] == text[..text.len() - 1] {
        return false;
    }
    unsafe { SetWindowTextW(hwnd, text.as_ptr()) };
    true
}
unsafe fn read_bounded(hwnd: HWND, max: u32) -> u32 {
    let mut b = [0u16; 16];
    let n = unsafe { GetWindowTextW(hwnd, b.as_mut_ptr(), 16) };
    let value = String::from_utf16_lossy(&b[..n.max(0) as usize])
        .parse::<u32>()
        .unwrap_or(0)
        .min(max);
    let current = String::from_utf16_lossy(&b[..n.max(0) as usize]);
    if value.to_string() != current {
        unsafe {
            set_text(hwnd, &value.to_string());
        }
    }
    value
}
unsafe fn configure_countdown() {
    if let Some(ui) = unsafe { UI } {
        let checked = unsafe { SendMessageW(ui.check, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize;
        let hours = unsafe { read_bounded(ui.hours, 999) };
        let minutes = unsafe { read_bounded(ui.minutes, 59) };
        let seconds = unsafe { read_bounded(ui.seconds, 59) };
        let secs = countdown_seconds(hours, minutes, seconds);
        STATE.with(|s| {
            s.borrow_mut()
                .configure_countdown(checked, secs, Instant::now())
        });
        unsafe { update_display(None) };
    }
}

unsafe fn toggle() {
    if STATE.with(|s| s.borrow().status != Status::Stopped) {
        unsafe { stop(false) };
        return;
    }
    if let Err(error) = unsafe { install_hooks() } {
        unsafe {
            message(&format!(
                "Unable to monitor input:\r\n{}",
                std::io::Error::from_raw_os_error(error as i32)
            ))
        };
        return;
    }
    let idle = unsafe { system_idle() };
    let now = Instant::now();
    let last = now
        .checked_sub(Duration::from_millis(u64::from(idle)))
        .unwrap_or(now);
    STATE.with(|s| s.borrow_mut().start(last, now));
    unsafe {
        tick();
    }
}
unsafe fn toggle_background() {
    let Some(ui) = (unsafe { UI }) else { return };
    let enable = !unsafe { BACKGROUND_TRANSPARENT };
    if let Err(error) = unsafe { apply_background_mode(ui.hwnd, enable) } {
        unsafe {
            message(&format!(
                "Unable to change background transparency:\r\n{}",
                std::io::Error::from_raw_os_error(error as i32)
            ));
        }
        return;
    }
    unsafe { BACKGROUND_TRANSPARENT = enable };
    unsafe {
        set_text(
            ui.background_toggle,
            if enable {
                "Restore opaque background"
            } else {
                "Make background transparent"
            },
        );
        RedrawWindow(
            ui.hwnd,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN,
        );
    }
}

unsafe fn apply_background_mode(hwnd: HWND, transparent: bool) -> Result<(), u32> {
    unsafe { SetLastError(0) };
    let previous = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let error = unsafe { GetLastError() };
    if previous == 0 && error != 0 {
        return Err(error);
    }
    let previous_style = previous as u32;
    let next_style = if transparent {
        previous_style | WS_EX_LAYERED
    } else {
        previous_style & !WS_EX_LAYERED
    };
    if previous_style == next_style && !transparent {
        return Ok(());
    }
    if previous_style != next_style {
        unsafe { SetLastError(0) };
        let prior = unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style as isize) };
        let error = unsafe { GetLastError() };
        if prior == 0 && error != 0 {
            return Err(error);
        }
    }
    if transparent
        && unsafe { SetLayeredWindowAttributes(hwnd, TRANSPARENCY_KEY, 255, LWA_COLORKEY) } == 0
    {
        let error = unsafe { GetLastError() };
        unsafe { restore_window_style(hwnd, previous) };
        return Err(if error == 0 { 31 } else { error });
    }
    if unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    } == 0
    {
        let error = unsafe { GetLastError() };
        unsafe { restore_window_style(hwnd, previous) };
        return Err(if error == 0 { 31 } else { error });
    }
    Ok(())
}

unsafe fn restore_window_style(hwnd: HWND, style: isize) {
    unsafe {
        SetLastError(0);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}
unsafe fn stop(done: bool) {
    cancel_worker(&WORKER_CANCEL);
    unsafe { destroy_hooks() };
    STATE.with(|s| s.borrow_mut().stop(done));
    unsafe { update_display(None) }
}
unsafe fn on_input() {
    let now = Instant::now();
    let was_moving = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let was_moving = state.status == Status::Moving;
        state.input(now);
        was_moving
    });
    if was_moving {
        unsafe { update_display(None) }
    }
}
unsafe fn tick() {
    let now = Instant::now();
    let mut expired = false;
    let mut begin = false;
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        let had = s.countdown_deadline.is_some();
        begin = s.tick(now);
        expired = had && s.countdown_done;
    });
    if expired {
        unsafe { stop(true) };
        return;
    }
    if begin {
        unsafe { start_movement() };
    }
    unsafe { update_display(None) }
}

unsafe fn update_display(initial: Option<Duration>) {
    let Some(ui) = (unsafe { UI }) else { return };
    let (state_status, last_input, done, deadline) = STATE.with(|s| {
        let s = s.borrow();
        (
            s.status,
            s.last_input,
            s.countdown_done,
            s.countdown_deadline,
        )
    });
    let idle = initial.unwrap_or_else(|| {
        if state_status == Status::Stopped {
            Duration::from_millis(u64::from(unsafe { system_idle() }))
        } else {
            Instant::now().saturating_duration_since(last_input)
        }
    });
    let status = match state_status {
        Status::Stopped => "Not Started",
        Status::Monitoring => "Monitoring",
        Status::Moving => "Moving (Auto)",
    };
    let (status_changed, button_changed) = unsafe {
        let status_changed = set_text(ui.status_badge, status);
        let button_changed = set_text(
            ui.button,
            if state_status == Status::Stopped {
                "Start (F10)"
            } else {
                "Stop (F10)"
            },
        );
        (status_changed, button_changed)
    };
    unsafe {
        if status_changed {
            invalidate_ui(ui.lamp);
            invalidate_ui(ui.status_badge);
        }
        if button_changed {
            invalidate_ui(ui.button);
        }
    }
    if unsafe { set_text(ui.idle_badge, &format!("IDLE {}", idle_clock(idle))) } {
        unsafe { invalidate_ui(ui.idle_badge) };
    }
    let left = until_move(idle);
    if unsafe { set_text(ui.in_time, &format!("In {}", clock(left))) } {
        unsafe { invalidate_ui(ui.in_time) };
    }
    let checked = unsafe { SendMessageW(ui.check, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize;
    let configured = countdown_seconds(
        unsafe { read_bounded(ui.hours, 999) },
        unsafe { read_bounded(ui.minutes, 59) },
        unsafe { read_bounded(ui.seconds, 59) },
    );
    let remaining = deadline.map(|d| d.saturating_duration_since(Instant::now()));
    let caption = countdown_caption(checked, done, remaining, configured);
    if unsafe { set_text(ui.check, &caption) } {
        unsafe { invalidate_ui(ui.check) };
    }
    let timer_needed = state_status != Status::Stopped || deadline.is_some();
    unsafe {
        if timer_needed && !TIMER_RUNNING {
            SetTimer(ui.hwnd, WM_TIMER_ID, 1000, None);
            TIMER_RUNNING = true
        } else if !timer_needed && TIMER_RUNNING {
            KillTimer(ui.hwnd, WM_TIMER_ID);
            TIMER_RUNNING = false
        }
    }
}
unsafe fn invalidate_ui(hwnd: HWND) {
    #[cfg(test)]
    TEST_UI_INVALIDATIONS.fetch_add(1, Ordering::SeqCst);
    unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
}
fn clock(d: Duration) -> String {
    let sec = d.as_secs();
    format!("{:02}:{:02}", sec / 60, sec % 60)
}
fn idle_clock(d: Duration) -> String {
    let sec = d.as_secs();
    format!("{:02}:{:02}:{:02}", sec / 3600, sec / 60 % 60, sec % 60)
}
fn until_move(idle: Duration) -> Duration {
    Duration::from_secs(180).saturating_sub(idle)
}
fn countdown_caption(
    checked: bool,
    done: bool,
    remaining: Option<Duration>,
    configured: u32,
) -> String {
    if !checked {
        "Stop after".to_owned()
    } else if done {
        "Stop done".to_owned()
    } else if let Some(remaining) = remaining {
        format!("Stop {}", idle_clock(remaining))
    } else if configured == 0 {
        "Set > 00:00".to_owned()
    } else {
        "Stop ready".to_owned()
    }
}
const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
unsafe fn background_brush() -> HBRUSH {
    unsafe {
        if BACKGROUND_TRANSPARENT {
            KEY_BRUSH
        } else {
            BG_BRUSH
        }
    }
}

unsafe fn init_theme_resources() -> bool {
    unsafe {
        BG_BRUSH = CreateSolidBrush(rgb(246, 248, 252));
        KEY_BRUSH = CreateSolidBrush(TRANSPARENCY_KEY);
        SURFACE_BRUSH = CreateSolidBrush(rgb(255, 255, 255));
        BUTTON_BORDER_BRUSH = CreateSolidBrush(rgb(13, 74, 185));
        CHECK_BRUSH = CreateSolidBrush(rgb(37, 99, 235));
        CHECK_HOT_BRUSH = CreateSolidBrush(rgb(232, 241, 255));
        IDLE_BORDER_BRUSH = CreateSolidBrush(rgb(174, 202, 240));
        LAMP_BRUSHES = [
            CreateSolidBrush(rgb(128, 128, 128)),
            CreateSolidBrush(rgb(50, 205, 50)),
            CreateSolidBrush(rgb(255, 0, 0)),
        ];
        STATUS_BRUSHES = [
            CreateSolidBrush(rgb(235, 238, 243)),
            CreateSolidBrush(rgb(220, 247, 228)),
            CreateSolidBrush(rgb(255, 228, 230)),
        ];
        let face = wide("Segoe UI");
        let font_height = |points: i32| -((points * DPI + 36) / 72);
        let font = |points, weight| {
            CreateFontW(
                font_height(points),
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            )
        };
        FONTS = [
            font(10, FW_NORMAL as i32),
            font(12, FW_SEMIBOLD as i32),
            font(11, FW_NORMAL as i32),
            font(10, FW_SEMIBOLD as i32),
            font(10, FW_SEMIBOLD as i32),
            font(12, FW_NORMAL as i32),
        ];
        !BG_BRUSH.is_null()
            && !KEY_BRUSH.is_null()
            && !SURFACE_BRUSH.is_null()
            && !BUTTON_BORDER_BRUSH.is_null()
            && !CHECK_BRUSH.is_null()
            && !CHECK_HOT_BRUSH.is_null()
            && !IDLE_BORDER_BRUSH.is_null()
            && !LAMP_BRUSHES[0].is_null()
            && !LAMP_BRUSHES[1].is_null()
            && !LAMP_BRUSHES[2].is_null()
            && !STATUS_BRUSHES[0].is_null()
            && !STATUS_BRUSHES[1].is_null()
            && !STATUS_BRUSHES[2].is_null()
            && !FONTS[0].is_null()
            && !FONTS[1].is_null()
            && !FONTS[2].is_null()
            && !FONTS[3].is_null()
            && !FONTS[4].is_null()
            && !FONTS[5].is_null()
    }
}

unsafe fn destroy_theme_resources() {
    unsafe {
        for brush in [
            BG_BRUSH,
            KEY_BRUSH,
            SURFACE_BRUSH,
            BUTTON_BORDER_BRUSH,
            CHECK_BRUSH,
            CHECK_HOT_BRUSH,
            IDLE_BORDER_BRUSH,
        ] {
            if !brush.is_null() {
                DeleteObject(brush as _);
            }
        }
        for brush in LAMP_BRUSHES {
            if !brush.is_null() {
                DeleteObject(brush as _);
            }
        }
        for brush in STATUS_BRUSHES {
            if !brush.is_null() {
                DeleteObject(brush as _);
            }
        }
        for font in FONTS {
            if !font.is_null() {
                DeleteObject(font as _);
            }
        }
    }
}

unsafe fn draw_primary_button(item: &DRAWITEMSTRUCT) {
    unsafe {
        let saved_dc = SaveDC(item.hDC);
        let rect = item.rcItem;
        let radius = (12 * DPI + 48) / 96;
        let region =
            CreateRoundRectRgn(rect.left, rect.top, rect.right, rect.bottom, radius, radius);
        let (top, bottom) = if item.itemState & ODS_SELECTED != 0 {
            ((24, 101, 235), (16, 74, 190))
        } else {
            ((58, 137, 255), (27, 91, 222))
        };
        FillRect(item.hDC, &rect, background_brush());
        SelectClipRgn(item.hDC, region);
        let vertices = [
            TRIVERTEX {
                x: rect.left,
                y: rect.top,
                Red: top.0 << 8,
                Green: top.1 << 8,
                Blue: top.2 << 8,
                Alpha: 0,
            },
            TRIVERTEX {
                x: rect.right,
                y: rect.bottom,
                Red: bottom.0 << 8,
                Green: bottom.1 << 8,
                Blue: bottom.2 << 8,
                Alpha: 0,
            },
        ];
        let gradient = GRADIENT_RECT {
            UpperLeft: 0,
            LowerRight: 1,
        };
        GradientFill(
            item.hDC,
            vertices.as_ptr(),
            vertices.len() as u32,
            &gradient as *const _ as *const _,
            1,
            GRADIENT_FILL_RECT_V,
        );
        FrameRgn(item.hDC, region, BUTTON_BORDER_BRUSH, 2, 2);
        SelectObject(item.hDC, FONTS[1] as _);
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, rgb(255, 255, 255));
        let mut text = [0u16; 64];
        let len = GetWindowTextW(item.hwndItem, text.as_mut_ptr(), text.len() as i32);
        let mut text_rect = rect;
        DrawTextW(
            item.hDC,
            text.as_ptr(),
            len,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        if item.itemState & ODS_FOCUS != 0 {
            let mut focus = rect;
            focus.left += 5;
            focus.top += 5;
            focus.right -= 5;
            focus.bottom -= 5;
            DrawFocusRect(item.hDC, &focus);
        }
        RestoreDC(item.hDC, saved_dc);
        DeleteObject(region as _);
    }
}

unsafe fn draw_transparency_toggle(item: &DRAWITEMSTRUCT) {
    unsafe {
        let saved_dc = SaveDC(item.hDC);
        let rect = item.rcItem;
        let scale = |value| (value * DPI + 48) / 96;
        FillRect(item.hDC, &rect, background_brush());
        let region = CreateRoundRectRgn(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            scale(6),
            scale(6),
        );
        if !BACKGROUND_TRANSPARENT {
            let border = if item.itemState & ODS_SELECTED != 0 {
                CHECK_BRUSH
            } else {
                IDLE_BORDER_BRUSH
            };
            let thickness = scale(if item.itemState & ODS_SELECTED != 0 {
                2
            } else {
                1
            });
            FrameRgn(item.hDC, region, border, thickness, thickness);
        }
        DeleteObject(region as _);

        let offset = if item.itemState & ODS_SELECTED != 0 {
            scale(1)
        } else {
            0
        };
        let side = scale(16);
        let cell = (side / 4).max(1);
        let left = rect.left + (rect.right - rect.left - side) / 2 + offset;
        let top = rect.top + (rect.bottom - rect.top - side) / 2 + offset;
        for row in 0..4 {
            for column in 0..4 {
                let cell_rect = RECT {
                    left: left + column * cell,
                    top: top + row * cell,
                    right: left + (column + 1) * cell,
                    bottom: top + (row + 1) * cell,
                };
                FillRect(
                    item.hDC,
                    &cell_rect,
                    if (row + column) % 2 == 0 {
                        SURFACE_BRUSH
                    } else {
                        BUTTON_BORDER_BRUSH
                    },
                );
            }
        }
        if item.itemState & ODS_FOCUS != 0 {
            let mut focus = rect;
            focus.left += scale(3);
            focus.top += scale(3);
            focus.right -= scale(3);
            focus.bottom -= scale(3);
            DrawFocusRect(item.hDC, &focus);
        }
        RestoreDC(item.hDC, saved_dc);
    }
}

unsafe fn draw_status_badge(item: &DRAWITEMSTRUCT) {
    let (index, text_color) = STATE.with(|s| match s.borrow().status {
        Status::Stopped => (0, rgb(82, 91, 105)),
        Status::Monitoring => (1, rgb(22, 101, 52)),
        Status::Moving => (2, rgb(185, 28, 28)),
    });
    let text_color = if unsafe { BACKGROUND_TRANSPARENT } {
        match index {
            0 => rgb(225, 230, 238),
            1 => rgb(103, 255, 156),
            _ => rgb(255, 121, 121),
        }
    } else {
        text_color
    };
    unsafe {
        draw_badge(
            item,
            STATUS_BRUSHES[index],
            std::ptr::null_mut(),
            text_color,
            FONTS[3],
        )
    };
}

unsafe fn draw_idle_badge(item: &DRAWITEMSTRUCT) {
    unsafe {
        let color = if BACKGROUND_TRANSPARENT {
            rgb(248, 250, 255)
        } else {
            rgb(23, 50, 91)
        };
        draw_badge(item, SURFACE_BRUSH, IDLE_BORDER_BRUSH, color, FONTS[4])
    };
}

unsafe fn draw_stop_checkbox(item: &NMCUSTOMDRAW) {
    unsafe {
        let saved_dc = SaveDC(item.hdc);
        let rect = item.rc;
        let scale = |value: i32| (value * DPI + 48) / 96;
        let checked = SendMessageW(item.hdr.hwndFrom, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
        let disabled = item.uItemState & CDIS_DISABLED != 0;
        let selected = item.uItemState & CDIS_SELECTED != 0;
        let hot = item.uItemState & CDIS_HOT != 0;
        FillRect(item.hdc, &rect, background_brush());

        let size = scale(20);
        let left = rect.left + scale(2);
        let top = rect.top + (rect.bottom - rect.top - size) / 2;
        let box_rect = RECT {
            left,
            top,
            right: left + size,
            bottom: top + size,
        };
        let radius = scale(5);
        let region = CreateRoundRectRgn(
            box_rect.left,
            box_rect.top,
            box_rect.right,
            box_rect.bottom,
            radius,
            radius,
        );
        let fill = if checked {
            if selected {
                BUTTON_BORDER_BRUSH
            } else {
                CHECK_BRUSH
            }
        } else if hot {
            CHECK_HOT_BRUSH
        } else {
            SURFACE_BRUSH
        };
        FillRgn(item.hdc, region, fill);
        FrameRgn(
            item.hdc,
            region,
            if disabled {
                STATUS_BRUSHES[0]
            } else {
                BUTTON_BORDER_BRUSH
            },
            scale(1),
            scale(1),
        );
        DeleteObject(region as _);

        if checked {
            let points = [
                POINT {
                    x: left + scale(4),
                    y: top + scale(10),
                },
                POINT {
                    x: left + scale(8),
                    y: top + scale(14),
                },
                POINT {
                    x: left + scale(16),
                    y: top + scale(5),
                },
            ];
            let pen = CreatePen(
                PS_SOLID,
                scale(2).max(1),
                if disabled {
                    rgb(225, 225, 225)
                } else {
                    rgb(255, 255, 255)
                },
            );
            let old_pen = SelectObject(item.hdc, pen as _);
            Polyline(item.hdc, points.as_ptr(), points.len() as i32);
            SelectObject(item.hdc, old_pen);
            DeleteObject(pen as _);
        }

        SelectObject(item.hdc, FONTS[5] as _);
        let mut text = [0u16; 64];
        let len = GetWindowTextW(item.hdr.hwndFrom, text.as_mut_ptr(), text.len() as i32);
        let mut text_rect = rect;
        text_rect.left = box_rect.right + scale(7);
        if BACKGROUND_TRANSPARENT {
            draw_outlined_text(
                item.hdc,
                text.as_ptr(),
                len,
                text_rect,
                DT_VCENTER | DT_SINGLELINE,
                if disabled {
                    rgb(175, 180, 190)
                } else {
                    rgb(248, 250, 255)
                },
                FONTS[5],
            );
        } else {
            SetBkMode(item.hdc, TRANSPARENT as i32);
            SetTextColor(
                item.hdc,
                if disabled {
                    rgb(152, 162, 179)
                } else {
                    rgb(23, 32, 51)
                },
            );
            DrawTextW(
                item.hdc,
                text.as_ptr(),
                len,
                &mut text_rect,
                DT_VCENTER | DT_SINGLELINE,
            );
        }
        if item.uItemState & CDIS_FOCUS != 0 {
            let mut focus = rect;
            focus.left += scale(1);
            focus.top += scale(1);
            focus.right -= scale(1);
            focus.bottom -= scale(1);
            DrawFocusRect(item.hdc, &focus);
        }
        RestoreDC(item.hdc, saved_dc);
    }
}

unsafe fn draw_badge(item: &DRAWITEMSTRUCT, fill: HBRUSH, border: HBRUSH, color: u32, font: HFONT) {
    unsafe {
        let saved_dc = SaveDC(item.hDC);
        let r = item.rcItem;
        if BACKGROUND_TRANSPARENT {
            FillRect(item.hDC, &r, KEY_BRUSH);
            let mut text = [0u16; 64];
            let len = GetWindowTextW(item.hwndItem, text.as_mut_ptr(), text.len() as i32);
            draw_outlined_text(
                item.hDC,
                text.as_ptr(),
                len,
                r,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                color,
                font,
            );
            RestoreDC(item.hDC, saved_dc);
            return;
        }
        let region = CreateRoundRectRgn(r.left, r.top, r.right, r.bottom, 10, 10);
        FillRect(item.hDC, &r, background_brush());
        FillRgn(item.hDC, region, fill);
        if !border.is_null() {
            FrameRgn(item.hDC, region, border, 1, 1);
        }
        SelectObject(item.hDC, font as _);
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, color);
        let mut text = [0u16; 64];
        let len = GetWindowTextW(item.hwndItem, text.as_mut_ptr(), text.len() as i32);
        let mut text_rect = r;
        DrawTextW(
            item.hDC,
            text.as_ptr(),
            len,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        RestoreDC(item.hDC, saved_dc);
        DeleteObject(region as _);
    }
}

unsafe fn draw_floating_static(item: &DRAWITEMSTRUCT) {
    unsafe {
        let saved_dc = SaveDC(item.hDC);
        let rect = item.rcItem;
        FillRect(item.hDC, &rect, background_brush());
        let text_color = match item.CtlID {
            116 if BACKGROUND_TRANSPARENT => rgb(142, 193, 255),
            116 => rgb(37, 99, 235),
            _ if BACKGROUND_TRANSPARENT => rgb(248, 250, 255),
            _ => rgb(23, 32, 51),
        };
        let font = if matches!(item.CtlID, 116) {
            FONTS[4]
        } else {
            FONTS[0]
        };
        let mut text = [0u16; 64];
        let len = GetWindowTextW(item.hwndItem, text.as_mut_ptr(), text.len() as i32);
        if BACKGROUND_TRANSPARENT {
            let mut outline_rect = rect;
            outline_rect.left += 1;
            draw_outlined_text(
                item.hDC,
                text.as_ptr(),
                len,
                outline_rect,
                DT_VCENTER | DT_SINGLELINE,
                text_color,
                font,
            );
        } else {
            SelectObject(item.hDC, font as _);
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(item.hDC, text_color);
            let mut text_rect = rect;
            DrawTextW(
                item.hDC,
                text.as_ptr(),
                len,
                &mut text_rect,
                DT_VCENTER | DT_SINGLELINE,
            );
        }
        RestoreDC(item.hDC, saved_dc);
    }
}

unsafe fn draw_outlined_text(
    hdc: HDC,
    text: *const u16,
    length: i32,
    rect: RECT,
    flags: u32,
    color: u32,
    font: HFONT,
) {
    unsafe {
        let saved_dc = SaveDC(hdc);
        SelectObject(hdc, font as _);
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, rgb(0, 0, 0));
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let mut outline = rect;
            outline.left += dx;
            outline.right += dx;
            outline.top += dy;
            outline.bottom += dy;
            DrawTextW(hdc, text, length, &mut outline, flags);
        }
        SetTextColor(hdc, color);
        let mut foreground = rect;
        DrawTextW(hdc, text, length, &mut foreground, flags);
        RestoreDC(hdc, saved_dc);
    }
}

unsafe fn draw_status_dot(item: &DRAWITEMSTRUCT) {
    let index = STATE.with(|s| match s.borrow().status {
        Status::Stopped => 0,
        Status::Monitoring => 1,
        Status::Moving => 2,
    });
    unsafe {
        let r = item.rcItem;
        let region = CreateEllipticRgn(r.left, r.top, r.right, r.bottom);
        FillRect(item.hDC, &r, background_brush());
        FillRgn(item.hDC, region, LAMP_BRUSHES[index]);
        DeleteObject(region as _);
    }
}
unsafe fn start_movement() {
    cancel_worker(&WORKER_CANCEL);
    let cancel = Arc::new(AtomicBool::new(false));
    replace_worker_cancel(&WORKER_CANCEL, cancel.clone());
    let hwnd = WINDOW.load(Ordering::SeqCst);
    unsafe {
        ShowWindow(hwnd as HWND, SW_RESTORE);
        SetForegroundWindow(hwnd as HWND)
    };
    thread::spawn(move || {
        let hwnd = hwnd as HWND;
        let cancel = cancel;
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            ^ 0x9e3779b97f4a7c15;
        loop {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let bounds = unsafe { primary_bounds() };
            let path = center_path(&build_azki_path(), bounds);
            for p in path {
                if cancel.load(Ordering::SeqCst) {
                    break;
                }
                let p = azu_sanpo::clamp(p, bounds);
                if unsafe { SetCursorPos(p.x, p.y) } == 0 {
                    if !cancel.swap(true, Ordering::SeqCst) {
                        unsafe {
                            PostMessageW(hwnd as HWND, WM_APP + 3, 1, GetLastError() as LPARAM)
                        };
                    }
                    break;
                }
                sleep_cancelable(Duration::from_millis(5 + (rng(&mut seed) % 5)), &cancel);
            }
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let inputs = [
                INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_LEFTDOWN,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: 0,
                            dwFlags: MOUSEEVENTF_LEFTUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];
            if unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) } != 2 {
                if !cancel.swap(true, Ordering::SeqCst) {
                    unsafe { PostMessageW(hwnd as HWND, WM_APP + 3, 2, 0) };
                }
                break;
            }
            if sleep_cancelable(Duration::from_secs(60), &cancel) {
                break;
            }
        }
    });
}
fn rng(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
fn sleep_cancelable(d: Duration, cancel: &AtomicBool) -> bool {
    let until = Instant::now() + d;
    while Instant::now() < until {
        if cancel.load(Ordering::SeqCst) {
            return true;
        }
        thread::sleep((until - Instant::now()).min(Duration::from_millis(20)));
    }
    false
}
unsafe fn primary_bounds() -> Bounds {
    Bounds {
        left: 0,
        top: 0,
        width: unsafe { GetSystemMetrics(SM_CXSCREEN) },
        height: unsafe { GetSystemMetrics(SM_CYSCREEN) },
    }
}
unsafe fn system_idle() -> u32 {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } != 0 {
        unsafe { GetTickCount().wrapping_sub(info.dwTime) }
    } else {
        0
    }
}
unsafe fn install_hooks() -> Result<(), u32> {
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) };
    if mouse.is_null() {
        return Err(unsafe { GetLastError() });
    }
    MOUSE_HOOK.store(mouse as isize, Ordering::SeqCst);
    let key = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_hook), module, 0) };
    if key.is_null() {
        let e = unsafe { GetLastError() };
        unsafe { destroy_hooks() };
        return Err(e);
    }
    KEY_HOOK.store(key as isize, Ordering::SeqCst);
    Ok(())
}
unsafe extern "system" fn mouse_hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let flags = unsafe { *(lp as *const MSLLHOOKSTRUCT) }.flags;
        if is_physical_mouse(flags) {
            signal_input()
        }
    }
    unsafe { CallNextHookEx(MOUSE_HOOK.load(Ordering::SeqCst) as _, code, wp, lp) }
}
unsafe extern "system" fn key_hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let flags = unsafe { *(lp as *const KBDLLHOOKSTRUCT) }.flags;
        if is_physical_keyboard(flags) {
            signal_input()
        }
    }
    unsafe { CallNextHookEx(KEY_HOOK.load(Ordering::SeqCst) as _, code, wp, lp) }
}
fn signal_input() {
    cancel_worker(&WORKER_CANCEL);
    let hwnd = WINDOW.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, WM_INPUT_EVENT, 0, 0);
        }
    }
}
unsafe fn destroy_hooks() {
    let m = MOUSE_HOOK.swap(0, Ordering::SeqCst);
    let k = KEY_HOOK.swap(0, Ordering::SeqCst);
    if m != 0 {
        unsafe { UnhookWindowsHookEx(m as _) };
    }
    if k != 0 {
        unsafe { UnhookWindowsHookEx(k as _) };
    }
}
unsafe fn message(text: &str) {
    let t = unsafe { wide(text) };
    let cap = unsafe { wide("MouseMoverApp") };
    unsafe {
        MessageBoxW(
            WINDOW.load(Ordering::SeqCst) as _,
            t.as_ptr(),
            cap.as_ptr(),
            MB_OK | MB_ICONERROR,
        )
    };
}

fn main() {
    unsafe {
        INSTANCE = GetModuleHandleW(std::ptr::null()) as isize;
        DPI = GetDpiForSystem() as i32;
        InitCommonControls();
        let class = wide("AzuSanpoWindow");
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN;
        let sc = |v: i32| (v * DPI + 48) / 96;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: sc(340),
            bottom: sc(130),
        };
        AdjustWindowRectEx(&mut rect, style, 0, WS_EX_TOPMOST);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: INSTANCE as _,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hIcon: LoadIconW(INSTANCE as _, 2 as _),
            hbrBackground: std::ptr::null_mut(),
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            startup_error("RegisterClassW", GetLastError());
            return;
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class.as_ptr(),
            wide("AZu Sanpo").as_ptr(),
            style,
            100,
            100,
            rect.right - rect.left,
            rect.bottom - rect.top,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            INSTANCE as _,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            startup_error("CreateWindowExW", GetLastError());
            return;
        }
        WINDOW.store(hwnd as isize, Ordering::SeqCst);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        let mut msg = MSG::default();
        let mut result = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
        while result > 0 {
            if IsDialogMessageW(hwnd, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            result = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
        }
        if result == -1 {
            startup_error("GetMessageW", GetLastError());
        }
        destroy_hooks();
    }
}

unsafe fn startup_error(api: &str, code: u32) {
    let message = wide(&format!(
        "{api} failed:\r\n{}",
        std::io::Error::from_raw_os_error(code as i32)
    ));
    let title = wide("MouseMoverApp");
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, GetDC, GetPixel, ReleaseDC,
    };

    unsafe fn has_non_key_pixel(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> bool {
        (top..bottom)
            .any(|y| (left..right).any(|x| unsafe { GetPixel(hdc, x, y) != TRANSPARENCY_KEY }))
    }

    #[test]
    fn idle_clock_and_move_countdown_show_hours_and_clamp_at_three_minutes() {
        assert_eq!(idle_clock(Duration::ZERO), "00:00:00");
        assert_eq!(idle_clock(Duration::from_secs(3600)), "01:00:00");
        assert_eq!(idle_clock(Duration::from_secs(360_001)), "100:00:01");
        assert_eq!(clock(until_move(Duration::from_secs(179))), "00:01");
        assert_eq!(clock(until_move(Duration::from_secs(180))), "00:00");
        assert_eq!(clock(until_move(Duration::from_secs(181))), "00:00");
    }

    #[test]
    fn countdown_feedback_is_compact_and_covers_enabled_states() {
        assert_eq!(countdown_caption(false, false, None, 10_800), "Stop after");
        assert_eq!(countdown_caption(true, false, None, 10_800), "Stop ready");
        assert_eq!(countdown_caption(true, false, None, 0), "Set > 00:00");
        assert_eq!(
            countdown_caption(true, false, Some(Duration::from_secs(10_800)), 10_800),
            "Stop 03:00:00"
        );
        assert_eq!(countdown_caption(true, true, None, 10_800), "Stop done");
    }

    type SubclassProc =
        Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;
    #[link(name = "comctl32")]
    extern "system" {
        fn DefSubclassProc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
        fn RemoveWindowSubclass(hwnd: HWND, callback: SubclassProc, id: usize) -> i32;
        fn SetWindowSubclass(hwnd: HWND, callback: SubclassProc, id: usize, data: usize) -> i32;
    }

    unsafe extern "system" fn count_text_writes(
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        if msg == WM_SETTEXT {
            (*(data as *const AtomicUsize)).fetch_add(1, Ordering::SeqCst);
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }

    #[test]
    fn set_text_skips_rewriting_an_unchanged_native_control() {
        unsafe {
            let class = wide("STATIC");
            let initial = wide("unchanged");
            let control = CreateWindowExW(
                0,
                class.as_ptr(),
                initial.as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!control.is_null());
            let writes = AtomicUsize::new(0);
            let subclass: SubclassProc = Some(count_text_writes);
            assert_ne!(
                SetWindowSubclass(control, subclass, 1, &writes as *const _ as usize),
                0
            );
            assert!(!set_text(control, "unchanged"));
            assert_eq!(writes.load(Ordering::SeqCst), 0);
            assert!(set_text(control, "changed"));
            assert_eq!(writes.load(Ordering::SeqCst), 1);
            RemoveWindowSubclass(control, subclass, 1);
            DestroyWindow(control);
        }
    }

    #[test]
    fn monitoring_input_flood_is_stable_and_transparent_controls_have_no_plates() {
        unsafe {
            INSTANCE = GetModuleHandleW(std::ptr::null()) as isize;
            DPI = 96;
            InitCommonControls();
            assert!(init_theme_resources());
            let class = wide("STATIC");
            let host = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("").as_ptr(),
                WS_POPUP,
                0,
                0,
                340,
                130,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!host.is_null());
            assert!(create_controls(host));
            let now = Instant::now();
            STATE.with(|s| s.borrow_mut().start(now - Duration::from_secs(1), now));
            update_display(Some(Duration::ZERO));

            let ui = UI.unwrap();
            let controls = [
                ui.button,
                ui.background_toggle,
                ui.lamp,
                ui.status_badge,
                ui.idle_badge,
                ui.in_time,
                ui.check,
            ];
            let writes = AtomicUsize::new(0);
            let subclass: SubclassProc = Some(count_text_writes);
            for (id, control) in controls.iter().enumerate() {
                assert_ne!(
                    SetWindowSubclass(*control, subclass, id + 1, &writes as *const _ as usize),
                    0
                );
            }
            writes.store(0, Ordering::SeqCst);
            TEST_UI_INVALIDATIONS.store(0, Ordering::SeqCst);
            let initial_input = STATE.with(|s| s.borrow().last_input);
            for _ in 0..100 {
                on_input();
            }
            let (status, last_input) = STATE.with(|s| (s.borrow().status, s.borrow().last_input));
            assert_eq!(status, Status::Monitoring);
            assert!(last_input > initial_input);
            assert_eq!(writes.load(Ordering::SeqCst), 0);
            assert_eq!(TEST_UI_INVALIDATIONS.load(Ordering::SeqCst), 0);

            let screen = GetDC(std::ptr::null_mut());
            assert!(!screen.is_null());
            let memory = CreateCompatibleDC(screen);
            assert!(!memory.is_null());
            let bitmap = CreateCompatibleBitmap(screen, 142, 26);
            assert!(!bitmap.is_null());
            let original = SelectObject(memory, bitmap as _);
            let mut draw: NMCUSTOMDRAW = std::mem::zeroed();
            draw.hdr.hwndFrom = ui.check;
            draw.hdc = memory;
            draw.rc = RECT {
                left: 0,
                top: 0,
                right: 142,
                bottom: 26,
            };
            SendMessageW(ui.check, BM_SETCHECK, BST_CHECKED as usize, 0);
            draw.uItemState = CDIS_HOT;
            draw_stop_checkbox(&draw);
            assert_eq!(GetPixel(memory, 5, 7), rgb(37, 99, 235));
            assert_eq!(GetPixel(memory, 10, 17), rgb(255, 255, 255));
            SendMessageW(ui.check, BM_SETCHECK, 0, 0);
            draw.uItemState = 0;
            draw_stop_checkbox(&draw);
            assert_eq!(GetPixel(memory, 5, 7), rgb(255, 255, 255));

            let status_item = DRAWITEMSTRUCT {
                CtlID: 102,
                hwndItem: ui.status_badge,
                hDC: memory,
                rcItem: RECT {
                    left: 0,
                    top: 0,
                    right: 108,
                    bottom: 24,
                },
                ..Default::default()
            };
            draw_status_badge(&status_item);
            assert_eq!(GetPixel(memory, 0, 0), rgb(246, 248, 252));

            BACKGROUND_TRANSPARENT = true;
            draw.uItemState = 0;
            draw_stop_checkbox(&draw);
            assert_eq!(GetPixel(memory, 140, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 37, 0, 142, 26));

            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            draw_status_badge(&status_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 0, 0, 108, 24));

            let idle_item = DRAWITEMSTRUCT {
                CtlID: 114,
                hwndItem: ui.idle_badge,
                hDC: memory,
                rcItem: RECT {
                    left: 0,
                    top: 0,
                    right: 136,
                    bottom: 24,
                },
                ..Default::default()
            };
            draw_idle_badge(&idle_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 0, 0, 136, 24));

            let mut in_item = DRAWITEMSTRUCT {
                CtlID: 116,
                hwndItem: ui.in_time,
                hDC: memory,
                rcItem: RECT {
                    left: 0,
                    top: 0,
                    right: 50,
                    bottom: 24,
                },
                ..Default::default()
            };
            draw_floating_static(&in_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 0, 0, 50, 24));

            in_item.CtlID = 113;
            in_item.hwndItem = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("h").as_ptr(),
                WS_POPUP,
                0,
                0,
                12,
                20,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!in_item.hwndItem.is_null());
            in_item.rcItem.right = 12;
            in_item.rcItem.bottom = 20;
            draw_floating_static(&in_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 0, 0, 12, 20));

            SetWindowTextW(in_item.hwndItem, wide("m").as_ptr());
            in_item.CtlID = 108;
            draw_floating_static(&in_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 0, 0, 12, 20));

            SetWindowTextW(in_item.hwndItem, wide("s").as_ptr());
            in_item.CtlID = 109;
            in_item.rcItem.right = 10;
            draw_floating_static(&in_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert!(has_non_key_pixel(memory, 0, 0, 10, 20));
            DestroyWindow(in_item.hwndItem);

            let dot_item = DRAWITEMSTRUCT {
                CtlID: 101,
                hwndItem: ui.lamp,
                hDC: memory,
                rcItem: RECT {
                    left: 0,
                    top: 0,
                    right: 14,
                    bottom: 14,
                },
                ..Default::default()
            };
            draw_status_dot(&dot_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);

            let icon_item = DRAWITEMSTRUCT {
                CtlID: 117,
                hwndItem: ui.background_toggle,
                hDC: memory,
                rcItem: RECT {
                    left: 0,
                    top: 0,
                    right: 26,
                    bottom: 28,
                },
                ..Default::default()
            };
            draw_transparency_toggle(&icon_item);
            assert_eq!(GetPixel(memory, 0, 0), TRANSPARENCY_KEY);
            assert_ne!(GetPixel(memory, 6, 7), TRANSPARENCY_KEY);

            BACKGROUND_TRANSPARENT = false;
            draw_status_badge(&status_item);
            assert_eq!(GetPixel(memory, 0, 0), rgb(246, 248, 252));
            SelectObject(memory, original);
            DeleteObject(bitmap as _);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);

            for (id, control) in controls.iter().enumerate() {
                RemoveWindowSubclass(*control, subclass, id + 1);
            }
            KillTimer(host, WM_TIMER_ID);
            TIMER_RUNNING = false;
            STATE.with(|s| s.borrow_mut().stop(false));
            DestroyWindow(host);
            UI = None;
            BACKGROUND_TRANSPARENT = false;
            destroy_theme_resources();
        }
    }

    #[test]
    fn background_transparency_uses_color_key_and_restores_window_style() {
        unsafe {
            let class = wide("STATIC");
            let host = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("").as_ptr(),
                WS_POPUP,
                0,
                0,
                10,
                10,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!host.is_null());
            let original = GetWindowLongPtrW(host, GWL_EXSTYLE);
            apply_background_mode(host, true).unwrap();
            assert_ne!(
                GetWindowLongPtrW(host, GWL_EXSTYLE) & WS_EX_LAYERED as isize,
                0
            );
            let mut key = 0;
            let mut alpha = 0;
            let mut flags = 0;
            assert_ne!(
                GetLayeredWindowAttributes(host, &mut key, &mut alpha, &mut flags),
                0
            );
            assert_eq!(key, TRANSPARENCY_KEY);
            assert_eq!(flags, LWA_COLORKEY);
            apply_background_mode(host, false).unwrap();
            assert_eq!(GetWindowLongPtrW(host, GWL_EXSTYLE), original);
            DestroyWindow(host);
        }
    }
}
