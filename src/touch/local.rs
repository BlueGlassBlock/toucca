use std::collections::HashMap;
use std::sync::Mutex;

use crate::config::*;
use crate::dprintln;
use crate::utils::{lo_word, DebugUnwrap};
use once_cell::sync::Lazy;

use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Input::Pointer::*;
use windows::Win32::UI::WindowsAndMessaging::*;

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
        unsafe {
            // Safety: CONFIG is "const"
            if let TouccaMode::Relative(TouccaRelativeConfig { map_lock, .. }) =
                &crate::CONFIG.touch.mode
            {
                let mut map_guard = map_lock.lock().dbg_unwrap();
                map_guard.remove(&ptr_info.pointerId);
            }
        }
    } else {
        let POINT { x, y } = ptr_info.ptPixelLocation;
        let rect = *_WINDOW_RECT.lock().dbg_unwrap();
        let center = ((rect.right + rect.left) / 2, (rect.bottom + rect.top) / 2);
        let (rel_x, rel_y) = (x - center.0, center.1 - y);
        guard.insert(ptr_info.pointerId, (rel_x, rel_y));
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

pub struct LocalTouchService();

impl LocalTouchService {
    #[allow(unused)] // TODO
    pub fn new() -> LocalTouchService {
        dprintln!("Toucca: Using LocalTouchService");
        unsafe {
            // Safety: guaranteed to be called once per process
            touch_init();
        }
        Self()
    }
}

impl super::TouchService for LocalTouchService {
    fn get_info(&self) -> super::PointerInfos {
        let rect = *_WINDOW_RECT.lock().dbg_unwrap();
        let radius = ((rect.right - rect.left) / 2).min((rect.bottom - rect.top) / 2);
        super::PointerInfos {
            map: _ACTIVE_POINTERS.lock().dbg_unwrap().clone(),
            radius: radius as u32,
        }
    }

    fn main_cycle(&self) {
        // Nothing to do here since we're hooking the WndProc
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}
