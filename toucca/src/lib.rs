mod log;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::thread::JoinHandle;

use toucca_lib::window::*;
use toucca_lib::{load_segatools_config, CONFIG};

use tracing::debug;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

static mut TRACING_INIT: bool = false;

#[no_mangle]
pub extern "system" fn mercury_io_get_api_version() -> u16 {
    unsafe {
        if !TRACING_INIT {
            log::init_log();
            TRACING_INIT = true;
        }
    }
    
    let version =
        option_env!("MERCURY_IO_VERSION").map_or(256, |v| v.parse::<u16>().unwrap_or(256));
    debug!("Returning API version: {}", version);

    version
}

static mut _SERIAL_HANDLE: Option<JoinHandle<()>> = None;

#[no_mangle]
pub extern "system" fn mercury_io_init() -> HRESULT {
    debug!("Reading config");
    load_segatools_config();
    unsafe {
        if CONFIG.is_serial && toucca_lib::is_mercury_process().unwrap() {
            _SERIAL_HANDLE = Some(toucca_lib::serial::start_serial());
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
        *OP_BTN_LOCK.lock().unwrap() = op_btn;
        *GAME_BTN_LOCK.lock().unwrap() = game_btn;
    }
    S_OK
}

#[no_mangle]
pub extern "system" fn mercury_io_get_opbtns(opbtn: *mut u8) {
    if let Some(mut ptr) = NonNull::new(opbtn) {
        unsafe {
            // Safety: relies on parent hook developer
            let op_btn_ref = ptr.as_mut();
            *op_btn_ref = *OP_BTN_LOCK.lock().unwrap();
        }
    }
}

#[no_mangle]
pub extern "system" fn mercury_io_get_gamebtns(gamebtn: *mut u8) {
    if let Some(mut ptr) = NonNull::new(gamebtn) {
        unsafe {
            // Safety: relies on parent hook developer
            let game_btn_ref = ptr.as_mut();
            *game_btn_ref = *GAME_BTN_LOCK.lock().unwrap();
        }
    }
}

static mut _TOUCH_INIT: bool = false;
static mut _HWND: HWND = HWND(0);

#[no_mangle]
pub extern "system" fn mercury_io_touch_init() -> HRESULT {
    unsafe {
        if !_TOUCH_INIT {
            let proc_id = GetCurrentProcessId();
            if let Some(handle) = get_window_handle(proc_id) {
                _HWND = handle;
                hook_wnd_proc(handle);
            }
            _TOUCH_INIT = true;
        }
    }
    S_OK
}

fn touch_loop(cell_pressed: &mut [bool; 240]) {
    cell_pressed.fill(false);
    let areas = get_active_areas();
    for area in areas {
        cell_pressed[area] = true;
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
    if let Some(handle) = &*_TOUCH_THREAD_HANDLE.lock().unwrap() {
        if !handle.is_finished() {
            return;
        }
    }
    let handle = std::thread::spawn(move || {
        debug!("Started touch poll thread");
        let mut cell_pressed: [bool; 240] = [false; 240];
        loop {
            touch_loop(&mut cell_pressed);
            callback(cell_pressed.as_mut_ptr());
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
    *_TOUCH_THREAD_HANDLE.lock().unwrap() = Some(handle);
}

#[no_mangle]
pub extern "system" fn mercury_io_touch_set_leds(_: *mut c_void) {}
