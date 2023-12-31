use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;

use crate::config::*;
use crate::lo_word;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::Pointer::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use tracing::{debug, instrument};

#[instrument(skip_all)]
unsafe extern "system" fn _enum_window(hwnd: HWND, param: LPARAM) -> BOOL {
    let hwnd_ptr_proc_id = (param.0 as *mut (Option<HWND>, u32)).as_mut().unwrap();
    let mut target_proc_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut target_proc_id as *mut u32));
    if target_proc_id == hwnd_ptr_proc_id.1 {
        let mut rect: RECT = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return TRUE;
        }
        debug!("Found target HWND, {:?}", rect);
        if (rect.right - rect.left) <= 0 || (rect.bottom - rect.top) <= 0 {
            return TRUE;
        }
        hwnd_ptr_proc_id.0 = Some(hwnd);
    }
    TRUE
}

#[instrument]
pub fn get_window_handle(proc_id: u32) -> Option<HWND> {
    let mut hwnd_proc_id: (Option<HWND>, u32) = (None, proc_id);
    let ptr_addr = &mut hwnd_proc_id as *mut (Option<HWND>, u32);
    unsafe {
        EnumWindows(Some(_enum_window), LPARAM(ptr_addr as isize)).unwrap();
        let hwnd = hwnd_proc_id.0;
        debug!("Get window handle of {}: {:?}", proc_id, hwnd);
        return hwnd;
    }
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

#[instrument(skip_all)]
fn update_pointer(_: HWND, param: WPARAM) {
    let mut ptr_info: POINTER_INFO = POINTER_INFO::default();
    unsafe {
        // Safety: GetPointerInfo is safe-ish
        if GetPointerInfo(lo_word(param) as u32, &mut ptr_info).is_err() {
            return;
        }
        ScreenToClient(ptr_info.hwndTarget, &mut ptr_info.ptPixelLocation);
    }
    let mut guard = _ACTIVE_POINTERS.lock().unwrap();
    if (ptr_info.pointerFlags & POINTER_FLAG_FIRSTBUTTON).0 == 0 {
        let touch_mode = unsafe { &crate::CONFIG.touch.mode };
        guard.remove(&ptr_info.pointerId);
        if let TouccaMode::Relative(TouccaRelativeConfig { map_lock, .. }) = touch_mode {
            let mut map_guard = map_lock.lock().unwrap();
            map_guard.remove(&ptr_info.pointerId);
        }
    } else {
        let POINT { x, y } = ptr_info.ptPixelLocation;
        guard.insert(ptr_info.pointerId, (x, y));
    }
}

static _WINDOW_RECT: Mutex<RECT> = Mutex::new(RECT {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
});

const WM_WINDOW_CHANGED_LIST: [u32; 2] = [WM_MOVE, WM_SIZE];

#[instrument(skip_all)]
fn update_window_rect(hwnd: HWND) {
    let mut rect: RECT = RECT::default();
    unsafe {
        // Safety: GetClientRect is safe-ish
        if let Err(e) = GetClientRect(hwnd, &mut rect) {
            debug!("Update window rect error: {:?}", e);
            return;
        }
    }
    *_WINDOW_RECT.lock().unwrap() = rect;
    debug!("Updated rect: {:?}", rect);
}

type WndProc = unsafe fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
static mut _FALLBACK_WND_PROC: WndProc = DefWindowProcW;

unsafe extern "system" fn wnd_proc_hook(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if !IsMouseInPointerEnabled().as_bool() {
        debug!("Enable mouse in pointer");
        if let Err(e) = EnableMouseInPointer(TRUE) {
            debug!("EnableMouseInPointer error: {:?}", e);
        }
    }
    // Safety: _FALLBACK_WND_PROC is set once per process
    if WM_POINTER_LIST.contains(&msg) {
        update_pointer(hwnd, w_param);
    } else if WM_WINDOW_CHANGED_LIST.contains(&msg) {
        update_window_rect(hwnd);
    }
    _FALLBACK_WND_PROC(hwnd, msg, w_param, l_param)
}

#[instrument(skip_all)]
pub unsafe fn hook_wnd_proc(hwnd: HWND) {
    update_window_rect(hwnd);
    let prev_wnd_proc = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
    if prev_wnd_proc != 0 {
        _FALLBACK_WND_PROC = std::mem::transmute(prev_wnd_proc);
    }
    #[allow(clippy::fn_to_numeric_cast)] // SetWindowLongPtrW just requires a long ptr
    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wnd_proc_hook as isize);
    debug!("Hooked WndProc for {:?}", hwnd);
}

fn to_polar(x: f64, y: f64) -> (f64, f64) {
    let r = (x * x + y * y).sqrt();
    let theta = y.atan2(x);
    (r, theta)
}

#[instrument]
fn parse_point(ptr_id: u32, abs_x: f64, abs_y: f64) -> Vec<usize> {
    use std::f64::consts::*;
    let radius_compensation: f64 = unsafe {
        crate::CONFIG.touch.radius_compensation
    } as f64;
    let rect = *_WINDOW_RECT.lock().unwrap();
    let center: (f64, f64) = (
        (rect.right + rect.left) as f64 / 2.0,
        (rect.bottom + rect.top) as f64 / 2.0,
    );
    debug!("center {}, {}", center.0, center.1);
    let radius = std::cmp::min(rect.right - rect.left, rect.bottom - rect.top) as f64 / 2.0
        + radius_compensation;
    let (rel_x, rel_y): (f64, f64) = (abs_x - center.0, center.1 - abs_y); // use center.y - abs_y to get Cartesian coordinate
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

    debug!("Section {}, dist {}", section, dist);
    if dist > radius {
        return vec![];
    }
    unsafe {
        let ring: usize = (crate::CONFIG.touch.divisions as f64 * dist / radius) as usize;
        debug!("Section {}, ring {}", section, ring);
        crate::CONFIG
            .touch
            .mode
            .to_cells(ptr_id, section, ring, crate::CONFIG.touch.pointer_radius)
    }
}

pub fn get_active_areas() -> HashSet<usize> {
    let mut touch_areas: HashSet<usize> = HashSet::new();
    let guard = _ACTIVE_POINTERS.lock().unwrap();
    for (ptr_id, (x, y)) in guard.iter() {
        let areas = parse_point(*ptr_id, *x as f64, *y as f64);
        touch_areas.extend(areas.into_iter());
    }
    touch_areas
}