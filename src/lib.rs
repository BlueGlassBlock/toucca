#![cfg(windows)]

mod config;
mod touch;
mod utils;

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::thread::JoinHandle;

use config::TouccaConfig;
use touch::PointerInfos;
use utils::DebugUnwrap;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

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
            TOUCH_SERVICE = Some(Box::new(touch::LocalTouchService::new())); // TODO
            _INIT = true;
        }
    }
    S_OK
}

fn to_polar(x: f64, y: f64) -> (f64, f64) {
    let r = (x * x + y * y).sqrt();
    let theta = y.atan2(x);
    (r, theta)
}

fn parse_point(ptr_id: u32, rel_x: f64, rel_y: f64, radius: f64) -> Vec<usize> {
    use std::f64::consts::*;
    let radius_compensation: f64 = unsafe {
        // Safety: CONFIG is "const"
        CONFIG.touch.radius_compensation
    } as f64;
    let radius = radius + radius_compensation;
    // rotate by 90 degrees
    let (rel_x, rel_y) = (rel_y, -rel_x);
    let (dist, angle) = to_polar(rel_x, rel_y);
    let section = {
        if angle < 0.0 {
            // right ring
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
        CONFIG
            .touch
            .mode
            .to_cells(ptr_id, section, ring, CONFIG.touch.pointer_radius)
    }
}

static mut TOUCH_SERVICE: Option<Box<dyn touch::TouchService>> = None;

fn touch_loop(cell_pressed: &mut [bool; 240]) {
    cell_pressed.fill(false);
    let PointerInfos { map, radius } = unsafe { TOUCH_SERVICE.as_ref() }.unwrap().get_info();
    for (ptr_id, (x, y)) in map.iter() {
        dprintln!(dbg, "Got touch {}, {}", x, y);
        let cells = parse_point(*ptr_id, *x as f64, *y as f64, radius as f64);
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
static _TOUCH_SERVICE_CYCLE_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

#[no_mangle]
pub extern "system" fn mercury_io_touch_start(callback: extern "C" fn(*mut bool)) {
    if let Some(handle) = &*_TOUCH_SERVICE_CYCLE_HANDLE.lock().dbg_unwrap() {
        if !handle.is_finished() {
            return;
        }
    }
    let handle = std::thread::spawn(|| unsafe {
        TOUCH_SERVICE.as_ref().unwrap().main_cycle();
    });
    *_TOUCH_SERVICE_CYCLE_HANDLE.lock().dbg_unwrap() = Some(handle);

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
