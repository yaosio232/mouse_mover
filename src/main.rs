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
    Foundation::{GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
        CreateEllipticRgn, CreateFontW, CreateRoundRectRgn, CreateSolidBrush, DeleteObject,
        DrawFocusRect, DrawTextW, FillRect, FillRgn, InvalidateRect, RestoreDC, SaveDC,
        SelectObject, SetBkColor, SetBkMode, SetTextColor, UpdateWindow, DT_CENTER, DT_SINGLELINE,
        DT_VCENTER, FW_NORMAL, FW_SEMIBOLD, HBRUSH, HDC, HFONT, OPAQUE, TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    System::SystemInformation::GetTickCount,
    System::SystemServices::{SS_CENTER, SS_LEFT, SS_OWNERDRAW},
    UI::{
        Controls::{
            InitCommonControls, BST_CHECKED, DRAWITEMSTRUCT, ODS_FOCUS, ODS_SELECTED, UDM_SETBUDDY,
            UDM_SETPOS32, UDM_SETRANGE32, UDS_ALIGNRIGHT, UDS_ARROWKEYS, UDS_SETBUDDYINT,
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
static WINDOW: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static KEY_HOOK: AtomicIsize = AtomicIsize::new(0);
static WORKER_CANCEL: WorkerCancel = Mutex::new(None);
static mut INSTANCE: isize = 0;
static mut DPI: i32 = 96;
static mut TIMER_RUNNING: bool = false;

#[derive(Clone, Copy)]
struct Ui {
    hwnd: HWND,
    button: HWND,
    lamp: HWND,
    status: HWND,
    detail: HWND,
    check: HWND,
    hours: HWND,
    minutes: HWND,
    seconds: HWND,
    countdown: HWND,
}
static mut UI: Option<Ui> = None;
static mut LAMP_BRUSHES: [HBRUSH; 3] = [std::ptr::null_mut(); 3];
static mut BG_BRUSH: HBRUSH = std::ptr::null_mut();
static mut SURFACE_BRUSH: HBRUSH = std::ptr::null_mut();
static mut DIVIDER_BRUSH: HBRUSH = std::ptr::null_mut();
static mut PRIMARY_BRUSH: HBRUSH = std::ptr::null_mut();
static mut PRESSED_BRUSH: HBRUSH = std::ptr::null_mut();
static mut FONTS: [HFONT; 5] = [std::ptr::null_mut(); 5];
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
                } else if id == 103
                    || ((id == 104 || id == 105 || id == 111) && notification == EN_CHANGE as u16)
                {
                    configure_countdown();
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
                FillRect(wp as HDC, &rect, BG_BRUSH);
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
        WM_CTLCOLORSTATIC => {
            let id = unsafe { GetDlgCtrlID(lp as HWND) };
            let color = match id {
                115 | 102 => rgb(23, 32, 51),
                116 | 114 => rgb(102, 112, 133),
                _ => rgb(71, 84, 103),
            };
            unsafe {
                SetTextColor(wp as HDC, color);
                if id == 117 {
                    SetBkColor(wp as HDC, rgb(226, 232, 240));
                    SetBkMode(wp as HDC, OPAQUE as i32);
                    DIVIDER_BRUSH as LRESULT
                } else {
                    SetBkMode(wp as HDC, TRANSPARENT as i32);
                    BG_BRUSH as LRESULT
                }
            }
        }
        WM_CTLCOLOREDIT => unsafe {
            SetTextColor(wp as HDC, rgb(23, 32, 51));
            SetBkColor(wp as HDC, rgb(255, 255, 255));
            SURFACE_BRUSH as LRESULT
        },
        WM_CTLCOLORBTN => unsafe {
            SetTextColor(wp as HDC, rgb(23, 32, 51));
            SetBkMode(wp as HDC, TRANSPARENT as i32);
            BG_BRUSH as LRESULT
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
            20,
            137,
            300,
            44,
            100,
        );
        let title = make("STATIC", "AZu Sanpo", SS_LEFT, 20, 15, 300, 30, 115);
        let subtitle = make(
            "STATIC",
            "Mouse movement, quietly.",
            SS_LEFT,
            20,
            47,
            300,
            18,
            116,
        );
        let lamp = make("STATIC", "", SS_OWNERDRAW, 20, 83, 12, 12, 101);
        let status = make("STATIC", "Not Started", SS_LEFT, 42, 79, 278, 22, 102);
        let detail = make(
            "STATIC",
            "Idle: 00:00    In: 03:00",
            SS_LEFT,
            20,
            103,
            300,
            18,
            114,
        );
        let check = make(
            "BUTTON",
            "Stop countdown",
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
            20,
            210,
            300,
            24,
            103,
        );
        let divider = make("STATIC", "", SS_LEFT, 20, 198, 300, 1, 117);
        let hour_label = make("STATIC", "Hours", SS_CENTER, 20, 241, 92, 18, 118);
        let min_label = make("STATIC", "Minutes", SS_CENTER, 124, 241, 92, 18, 119);
        let sec_label_text = make("STATIC", "Seconds", SS_CENTER, 228, 241, 92, 18, 120);
        let hours = make(
            "EDIT",
            "3",
            WS_BORDER | WS_TABSTOP | ES_NUMBER as u32 | ES_CENTER as u32,
            20,
            262,
            92,
            34,
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
            title,
            subtitle,
            lamp,
            status,
            detail,
            check,
            divider,
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
            124,
            262,
            92,
            34,
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
            228,
            262,
            92,
            34,
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
        let countdown = make(
            "STATIC",
            "Stop countdown: Off",
            SS_LEFT,
            20,
            309,
            300,
            19,
            110,
        );
        if countdown.is_null() {
            return false;
        }
        SendMessageW(title, WM_SETFONT, FONTS[1] as usize, 1);
        SendMessageW(status, WM_SETFONT, FONTS[2] as usize, 1);
        SendMessageW(subtitle, WM_SETFONT, FONTS[3] as usize, 1);
        for edit in [hours, minutes, sec_edit] {
            SendMessageW(edit, WM_SETFONT, FONTS[4] as usize, 1);
        }
        UI = Some(Ui {
            hwnd,
            button,
            lamp,
            status,
            detail,
            check,
            hours,
            minutes,
            seconds: sec_edit,
            countdown,
        });
    }
    true
}

unsafe fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe fn set_text(hwnd: HWND, text: &str) {
    let text = unsafe { wide(text) };
    unsafe { SetWindowTextW(hwnd, text.as_ptr()) };
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
        unsafe { set_text(hwnd, &value.to_string()) }
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
unsafe fn stop(done: bool) {
    cancel_worker(&WORKER_CANCEL);
    unsafe { destroy_hooks() };
    STATE.with(|s| s.borrow_mut().stop(done));
    unsafe { update_display(None) }
}
unsafe fn on_input() {
    let now = Instant::now();
    STATE.with(|s| s.borrow_mut().input(now));
    unsafe { update_display(None) }
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
    unsafe {
        set_text(ui.status, status);
        set_text(
            ui.button,
            if state_status == Status::Stopped {
                "Start (F10)"
            } else {
                "Stop (F10)"
            },
        );
    }
    unsafe {
        InvalidateRect(ui.lamp, std::ptr::null(), 1);
        InvalidateRect(ui.button, std::ptr::null(), 1);
    }
    let left = Duration::from_secs(180).saturating_sub(idle);
    let info = format!("Idle: {}    In: {}", clock(idle), clock(left));
    unsafe { set_text(ui.detail, &info) };
    let countdown = if unsafe { SendMessageW(ui.check, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize
    {
        if done {
            "Done".to_owned()
        } else if let Some(d) = deadline {
            clock(d.saturating_duration_since(Instant::now()))
        } else if countdown_seconds(
            unsafe { read_bounded(ui.hours, 999) },
            unsafe { read_bounded(ui.minutes, 59) },
            unsafe { read_bounded(ui.seconds, 59) },
        ) == 0
        {
            "set > 00:00".to_owned()
        } else {
            "Ready".to_owned()
        }
    } else {
        "Off".to_owned()
    };
    unsafe { set_text(ui.countdown, &format!("Stop countdown: {countdown}")) };
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
fn clock(d: Duration) -> String {
    let sec = d.as_secs();
    format!("{:02}:{:02}", sec / 60, sec % 60)
}
const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}

unsafe fn init_theme_resources() -> bool {
    unsafe {
        BG_BRUSH = CreateSolidBrush(rgb(246, 248, 252));
        SURFACE_BRUSH = CreateSolidBrush(rgb(255, 255, 255));
        DIVIDER_BRUSH = CreateSolidBrush(rgb(226, 232, 240));
        PRIMARY_BRUSH = CreateSolidBrush(rgb(37, 99, 235));
        PRESSED_BRUSH = CreateSolidBrush(rgb(29, 78, 216));
        LAMP_BRUSHES = [
            CreateSolidBrush(rgb(128, 128, 128)),
            CreateSolidBrush(rgb(50, 205, 50)),
            CreateSolidBrush(rgb(255, 0, 0)),
        ];
        let face = wide("Segoe UI");
        let font_height = |points: i32| -((points * DPI + 36) / 72);
        FONTS = [
            CreateFontW(
                font_height(10),
                0,
                0,
                0,
                FW_NORMAL as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            ),
            CreateFontW(
                font_height(18),
                0,
                0,
                0,
                FW_SEMIBOLD as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            ),
            CreateFontW(
                font_height(11),
                0,
                0,
                0,
                FW_SEMIBOLD as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            ),
            CreateFontW(
                font_height(9),
                0,
                0,
                0,
                FW_NORMAL as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            ),
            CreateFontW(
                font_height(12),
                0,
                0,
                0,
                FW_NORMAL as i32,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                face.as_ptr(),
            ),
        ];
        !BG_BRUSH.is_null()
            && !SURFACE_BRUSH.is_null()
            && !DIVIDER_BRUSH.is_null()
            && !PRIMARY_BRUSH.is_null()
            && !PRESSED_BRUSH.is_null()
            && !LAMP_BRUSHES[0].is_null()
            && !LAMP_BRUSHES[1].is_null()
            && !LAMP_BRUSHES[2].is_null()
            && !FONTS[0].is_null()
            && !FONTS[1].is_null()
            && !FONTS[2].is_null()
            && !FONTS[3].is_null()
            && !FONTS[4].is_null()
    }
}

unsafe fn destroy_theme_resources() {
    unsafe {
        for brush in [
            BG_BRUSH,
            SURFACE_BRUSH,
            DIVIDER_BRUSH,
            PRIMARY_BRUSH,
            PRESSED_BRUSH,
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
        let brush = if item.itemState & ODS_SELECTED != 0 {
            PRESSED_BRUSH
        } else {
            PRIMARY_BRUSH
        };
        FillRgn(item.hDC, region, brush);
        SelectObject(item.hDC, FONTS[2] as _);
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

unsafe fn draw_status_dot(item: &DRAWITEMSTRUCT) {
    let index = STATE.with(|s| match s.borrow().status {
        Status::Stopped => 0,
        Status::Monitoring => 1,
        Status::Moving => 2,
    });
    unsafe {
        let r = item.rcItem;
        let region = CreateEllipticRgn(r.left, r.top, r.right, r.bottom);
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
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let sc = |v: i32| (v * DPI + 48) / 96;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: sc(340),
            bottom: sc(344),
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
