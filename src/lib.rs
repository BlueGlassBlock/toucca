#![cfg(windows)]

mod config;
mod utils;

use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::thread::JoinHandle;

use config::TouccaConfig;
use once_cell::sync::Lazy;
use utils::{lo_word, DebugUnwrap};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::Input::Pointer::*;
use windows::Win32::UI::WindowsAndMessaging::*;

#[no_mangle]
pub extern "system" fn mercury_io_get_api_version() -> u16 {
    let version =
        option_env!("MERCURY_IO_VERSION").map_or_else(|| 256, |v| v.parse::<u16>().unwrap_or(256));
    dprintln!("Returning API version: {}", version);

    version
}

static mut CONFIG: TouccaConfig = TouccaConfig::default();

#[no_mangle]
pub extern "system" fn mercury_io_init() -> HRESULT {
    std::panic::set_hook(Box::new(|info| {
        dprintln!("Toucca panicked: {}", info);
    }));

    dprintln!("Reading config");
    unsafe {
        // Safety: CONFIG is initialized once per process (daemon and game process)
        CONFIG = TouccaConfig::load(&HSTRING::from(".\\segatools.ini"));
        if let &config::TouccaMode::Relative(..) = &CONFIG.touch.mode {
            panic!("Relative touch mode is not supported");
        }
    }

    S_OK
}

enum OpBtn {
    Test = 0x01,
    Service = 0x02,
    Coin = 0x04,
}

enum GameBtn {
    VolUp = 0x01,
    VolDown = 0x02,
}

static OP_BTN_LOCK: Mutex<u8> = Mutex::new(0);
static GAME_BTN_LOCK: Mutex<u8> = Mutex::new(0);

#[no_mangle]
pub extern "system" fn mercury_io_poll() -> HRESULT {
    let mut op_btn: u8 = 0;
    let mut game_btn: u8 = 0;
    unsafe {
        // Safety: CONFIG is "const" & GetAsyncKeyState is safe-ish
        if GetAsyncKeyState(CONFIG.vk_test) != 0 {
            op_btn |= OpBtn::Test as u8;
        }
        if GetAsyncKeyState(CONFIG.vk_service) != 0 {
            op_btn |= OpBtn::Service as u8;
        }
        if GetAsyncKeyState(CONFIG.vk_coin) != 0 {
            op_btn |= OpBtn::Coin as u8;
        }
        if GetAsyncKeyState(CONFIG.vk_vol_up) != 0 {
            game_btn |= GameBtn::VolUp as u8;
        }
        if GetAsyncKeyState(CONFIG.vk_vol_down) != 0 {
            game_btn |= GameBtn::VolDown as u8;
        }
        *OP_BTN_LOCK.lock().dbg_unwrap() = op_btn;
        *GAME_BTN_LOCK.lock().dbg_unwrap() = game_btn;
    }
    S_OK
}

#[no_mangle]
pub extern "system" fn mercury_io_get_opbtns(opbtn: *mut u8) {
    if let Some(mut ptr) = NonNull::new(opbtn) {
        unsafe {
            // Safety: relies on parent hook developer
            let op_btn_ref = ptr.as_mut();
            *op_btn_ref = *OP_BTN_LOCK.lock().dbg_unwrap();
        }
    }
}

#[no_mangle]
pub extern "system" fn mercury_io_get_gamebtns(gamebtn: *mut u8) {
    if let Some(mut ptr) = NonNull::new(gamebtn) {
        unsafe {
            // Safety: relies on parent hook developer
            let game_btn_ref = ptr.as_mut();
            *game_btn_ref = *GAME_BTN_LOCK.lock().dbg_unwrap();
        }
    }
}

static mut _INIT: bool = false;
#[no_mangle]
pub extern "system" fn mercury_io_touch_init() -> HRESULT {
    unsafe {
        // Safety: Only initialized once per process
        if !_INIT {
            dprintln!("Toucca: Touch init");
            touch_init();
            _INIT = true;
        }
    }
    S_OK
}

static mut _HWND: HWND = HWND(0);
unsafe extern "system" fn _enum_window(hwnd: HWND, proc_id: LPARAM) -> BOOL {
    let mut target_proc_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut target_proc_id as *mut u32));
    if target_proc_id == proc_id.0 as u32 {
        let mut rect: RECT = RECT::default();
        GetWindowRect(hwnd, &mut rect).dbg_unwrap();
        dprintln!("Toucca: Found target HWND, {:?}", rect);
        if (rect.right - rect.left) <= 0 || (rect.bottom - rect.top) <= 0 {
            return TRUE;
        }
        _HWND = hwnd;
    }
    TRUE
}

unsafe fn get_window_handle() -> HWND {
    let proc_id: u32 = GetCurrentProcessId();

    EnumWindows(Some(_enum_window), LPARAM(proc_id as isize)).dbg_unwrap();
    dprintln!("Get window handle: {:?}", _HWND);
    let mut guard = _WINDOW_RECT.lock().dbg_unwrap();
    GetWindowRect(_HWND, &mut *guard).dbg_unwrap();
    drop(guard);
    _HWND
}

static _ACTIVE_POINTERS: Lazy<Mutex<HashMap<u32, (i32, i32)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
const WM_POINTER_LIST: [u32; 7] = [
    WM_POINTERDOWN,
    WM_POINTERUP,
    WM_POINTERCAPTURECHANGED,
    WM_POINTERUPDATE,
    WM_POINTERENTER,
    WM_POINTERLEAVE,
    WM_POINTERACTIVATE,
];
fn update_pointer(_: HWND, param: WPARAM) {
    let mut ptr_info: POINTER_INFO = POINTER_INFO::default();
    unsafe {
        // Safety: GetPointerInfo is safe-ish
        if GetPointerInfo(lo_word(param) as u32, &mut ptr_info).is_err() {
            return;
        }
    }
    let mut guard = _ACTIVE_POINTERS.lock().dbg_unwrap();
    if (ptr_info.pointerFlags & POINTER_FLAG_FIRSTBUTTON).0 == 0 {
        guard.remove(&ptr_info.pointerId);
    } else {
        let POINT { x, y } = ptr_info.ptPixelLocation;
        guard.insert(ptr_info.pointerId, (x, y));
    }
    drop(guard);
}

static _WINDOW_RECT: Mutex<RECT> = Mutex::new(RECT {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
});
const WM_WINDOW_CHANGED_LIST: [u32; 2] = [WM_MOVE, WM_SIZE];
fn update_window_rect(hwnd: HWND, _: LPARAM) {
    let mut rect: RECT = RECT::default();
    unsafe {
        // Safety: GetWindowRect is safe-ish
        if let Err(e) = GetWindowRect(hwnd, &mut rect) {
            dprintln!("Toucca update window rect error: {:?}", e);
            return;
        }
    }
    *_WINDOW_RECT.lock().dbg_unwrap() = rect;
    dprintln!(dbg, "Updated rect: {:?}", rect);
}
type WndProc = unsafe fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
static mut _FALLBACK_WND_PROC: WndProc = DefWindowProcW;

unsafe extern "system" fn wnd_proc_hook(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    // Safety: _FALLBACK_WND_PROC is set once per process
    if WM_POINTER_LIST.contains(&msg) {
        update_pointer(hwnd, w_param);
    } else if WM_WINDOW_CHANGED_LIST.contains(&msg) {
        update_window_rect(hwnd, l_param);
    }
    _FALLBACK_WND_PROC(hwnd, msg, w_param, l_param)
}

unsafe fn touch_init() -> HWND {
    // Safety: guaranteed to be called once per process
    let hwnd: HWND = get_window_handle();
    if !IsMouseInPointerEnabled().as_bool() {
        dprintln!("Toucca: Enable mouse in pointer");
        EnableMouseInPointer(TRUE).dbg_unwrap();
    }
    let prev_wnd_proc = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
    if prev_wnd_proc != 0 {
        _FALLBACK_WND_PROC = std::mem::transmute(prev_wnd_proc);
    }
    #[allow(clippy::fn_to_numeric_cast)] // SetWindowLongPtrW just requires a long ptr
    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wnd_proc_hook as isize);
    dprintln!("Hooked WndProc for {:?}", hwnd);
    hwnd
}

fn to_polar(x: f64, y: f64) -> (f64, f64) {
    let r = (x * x + y * y).sqrt();
    let theta = y.atan2(x);
    (r, theta)
}

fn parse_point(abs_x: f64, abs_y: f64) -> Vec<usize> {
    use std::f64::consts::*;
    let rect = *_WINDOW_RECT.lock().dbg_unwrap();
    let center: (f64, f64) = (
        (rect.right + rect.left) as f64 / 2.0,
        (rect.bottom + rect.top) as f64 / 2.0,
    );
    let radius = std::cmp::min(rect.right - rect.left, rect.bottom - rect.top) as f64 / 2.0 + 30.0;
    let (rel_x, rel_y): (f64, f64) = (abs_x - center.0, center.1 - abs_y); // use center.y - abs_y to get Cartesian coordinate
                                                                           // rotate by 90 degrees
    let (rel_x, rel_y) = (rel_y, -rel_x);
    let (dist, angle) = to_polar(rel_x, rel_y);
    let section = {
        if angle < 0.0
        // right ring
        {
            (-angle) / PI * 30.0
        } else {
            angle / PI * 30.0 + 30.0
        }
    } as usize;

    dprintln!(dbg, "Got section {}, dist {}", section, dist);
    if dist > radius {
        return vec![];
    }
    unsafe {
        let ring: usize = (CONFIG.touch.divisions as f64 * dist / radius) as usize;
        dprintln!(dbg, "Got section {}, ring {}", section, ring);
        CONFIG.touch.mode.to_cells(section, ring)
    }
}

fn touch_loop(cell_pressed: &mut [bool; 240]) {
    cell_pressed.fill(false);
    let read_guard = _ACTIVE_POINTERS.lock().dbg_unwrap();
    for (_, (x, y)) in read_guard.iter() {
        dprintln!(dbg, "Got touch {}, {}", x, y);
        let cells = parse_point(*x as f64, *y as f64);
        for cell in cells {
            cell_pressed[cell] = true;
        }
    }
    for (i, cell_state) in cell_pressed.iter_mut().enumerate() {
        unsafe {
            // Safety: CONFIG is "const" & GetAsyncKeyState is safe-ish
            if GetAsyncKeyState(CONFIG.vk_cell[i]) != 0 {
                *cell_state = true;
            }
        }
    }
}

static _TOUCH_THREAD_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

#[no_mangle]
pub extern "system" fn mercury_io_touch_start(callback: extern "C" fn(*mut bool)) {
    if let Some(handle) = &*_TOUCH_THREAD_HANDLE.lock().dbg_unwrap() {
        if !handle.is_finished() {
            return;
        }
    }
    let handle = std::thread::spawn(move || {
        dprintln!("Started touch poll thread");
        let mut cell_pressed: [bool; 240] = [false; 240];
        loop {
            touch_loop(&mut cell_pressed);
            callback(cell_pressed.as_mut_ptr());
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
    *_TOUCH_THREAD_HANDLE.lock().dbg_unwrap() = Some(handle);
}

#[no_mangle]
pub extern "system" fn mercury_io_touch_set_leds(_: *mut c_void) {}
