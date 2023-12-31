mod constant;
mod pack;
mod privileged;

use std::collections::HashSet;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use serial::SerialPort;
use serialport as serial;
use tracing::{debug, error, info, instrument};

use toucca::window::*;
use toucca::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;
use windows::Win32::Foundation::*;

struct TouccaState {
    ports: [serial::COMPort; 2],
    startup_complete: bool,
    touch_areas: Arc<Mutex<HashSet<usize>>>,
    exit: Arc<Mutex<bool>>,
}

impl TouccaState {
    fn new() -> Self {
        info!("Trying to start serial ports");
        let com_l = serial::new("COM5", 115200)
            .open_native()
            .expect("Failed to open COM5");
        let com_r = serial::new("COM6", 115200)
            .open_native()
            .expect("Failed to open COM6");
        info!("Opened serial ports");
        Self {
            ports: [com_l, com_r],
            startup_complete: false,
            touch_areas: Default::default(),
            exit: Default::default(),
        }
    }

    #[tracing::instrument(skip_all, fields(side = side))]
    fn make_resp(side: usize, head: u8, data: &[u8]) -> (Option<bool>, Option<Vec<u8>>) {
        let mut startup_complete = None;
        let mut resp_data = None;
        match head {
            constant::GET_SYNC_BOARD_VER => {
                debug!("GET_SYNC_BOARD_VER");
                startup_complete = Some(false);
                let mut buf = vec![];
                buf.push(head);
                buf.extend(constant::SYNC_BOARD_VER.as_bytes());
                buf.push(44);
                resp_data = Some(buf);
            }
            constant::NEXT_READ => {
                debug!("NEXT_READ");
                let mut buf: Option<Vec<u8>> = None;
                match data.get(2).unwrap_or(&0) {
                    0x30 => {
                        debug!("READ_1");
                        buf = Some(constant::READ_1.as_bytes().into());
                    }
                    0x31 => {
                        debug!("READ_2");
                        buf = Some(constant::READ_2.as_bytes().into());
                    }
                    0x33 => {
                        debug!("READ_3");
                        buf = Some(constant::READ_3.as_bytes().into());
                    }
                    _ => debug!("Extra read"),
                }
                if let Some(mut buf) = buf {
                    buf.push(pack::checksum(&buf));
                    resp_data = Some(buf);
                }
                startup_complete = Some(false);
            }
            constant::GET_UNIT_BOARD_VER => {
                debug!("GET_UNIT_BOARD_VER");
                let mut buf = vec![];
                buf.push(head);
                buf.extend(constant::SYNC_BOARD_VER.as_bytes());
                buf.extend(if side == 0 { "R" } else { "L" }.as_bytes()); // side byte
                buf.extend(constant::UNIT_BOARD_VER.as_bytes().repeat(6));
                buf.push(if side == 0 {
                    constant::UNIT_R
                } else {
                    constant::UNIT_L
                }); // unit checksum
                resp_data = Some(buf);
                startup_complete = Some(false);
            }
            constant::MYSTERY_1 => {
                debug!("MYSTERY_1");
                startup_complete = Some(false);
                resp_data = Some(constant::DATA_162.into());
            }
            constant::MYSTERY_2 => {
                debug!("MYSTERY_2");
                startup_complete = Some(false);
                resp_data = Some(constant::DATA_148.into());
            }
            constant::START_AUTO_SCAN => {
                debug!("START_AUTO_SCAN");
                startup_complete = Some(true);
                resp_data = Some(constant::DATA_201.into());
                // "start touch thread"
            }
            constant::BEGIN_WRITE => debug!("BEGIN_WRITE"),
            constant::NEXT_WRITE => debug!("NEXT_WRITE"),
            constant::BAD_IN_BYTE => {
                debug!("BAD_IN_BYTE");
                startup_complete = Some(false);
            }
            _ => {}
        }
        (startup_complete, resp_data)
    }

    fn read_and_update(&mut self) -> serial::Result<()> {
        for (side, port) in self.ports.iter_mut().enumerate() {
            let to_read = port.bytes_to_read()? as usize;
            if to_read == 0 {
                continue;
            }
            let head = {
                let mut buf = [0; 1];
                port.read(&mut buf)?;
                buf[0]
            };
            let mut buf = vec![0; to_read - 1];
            port.read(&mut buf)?;
            let (startup_complete, resp) = Self::make_resp(side, head, &buf);
            if let Some(startup_complete) = startup_complete {
                self.startup_complete = startup_complete;
            }
            if let Some(resp) = resp {
                port.write_all(&resp)?;
            }
        }
        Ok(())
    }

    fn update_touch(&mut self) -> serial::Result<()> {
        if !self.startup_complete {
            return Ok(());
        }
        let mut packs: [pack::Pack; 2] = [[0; 36], [0; 36]];
        let guard = self
            .touch_areas
            .lock()
            .expect("touch_recv lock is poisoned!");
        for &area in guard.iter() {
            let side = if area >= 120 { 0 } else { 1 };
            let index = area % 120;
            pack::set(&mut packs[side], index, true);
        }
        drop(guard);
        for (side, port) in self.ports.iter_mut().enumerate() {
            let pack = pack::prepare(packs[side]);
            port.write_all(&pack)?;
        }
        Ok(())
    }

    fn cycle(&mut self) {
        use std::thread::sleep;
        use std::time::Duration;
        while !self.exit.lock().expect("exit lock is poisoned!").clone() {
            if let Err(e) = self.read_and_update() {
                error!("Error reading and updating: {}", e);
            }
            if let Err(e) = self.update_touch() {
                error!("Error updating touch info to game: {}", e);
            }
            sleep(Duration::from_millis(16));
        }
    }
}

unsafe fn search_mercury_once(token: HANDLE, areas: &Arc<Mutex<HashSet<usize>>>, proc_id: &mut u32, hwnd: &mut Option<HWND>) -> Result<()> {
    if *proc_id == 0 {
        *proc_id = privileged::find_mercury_proc(token)?;
    }
    if *proc_id != 0 && hwnd.is_none() {
        if let Some(handle) = get_window_handle(*proc_id) {
            *hwnd = Some(handle);
            hook_wnd_proc(handle);
        } else {
            *proc_id = 0;
        }
    }
    // check whether the process is still alive
    if *proc_id != 0 {
        if let Err(e) = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, *proc_id) {
            *proc_id = 0;
            *hwnd = None;
            info!("Invalid process: {e}");
        }
    }
    // check whether the window is still alive
    if let Some(handle) = *hwnd {
        if !IsWindow(handle).as_bool() {
            *proc_id = 0;
            *hwnd = None;
        }
    }
    *areas.lock().unwrap() = window::get_active_areas();
    std::thread::sleep(std::time::Duration::from_micros(8));
    Ok(())
}

#[instrument(skip_all)]
unsafe fn window_search_cycle(token: HANDLE, areas: Arc<Mutex<HashSet<usize>>>, exit: Arc<Mutex<bool>>) {
    let mut proc_id: u32 = 0;
    let mut hwnd: Option<HWND> = None;
    while !exit.lock().expect("exit lock is poisoned!").clone() {
        if let Err(e) = search_mercury_once(token, &areas, &mut proc_id, &mut hwnd) {
            proc_id = 0;
            hwnd = None;
            error!("Error searching for Mercury: {}", e);
        }
    }
}

fn setup_log() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_thread_names(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}

fn main() {
    setup_log();
    privileged::check_privilege();
    load_segatools_config();
    let mut state = TouccaState::new();
    let (touch_areas, exit) = (state.touch_areas.clone(), state.exit.clone());

    let _ctrlc_exit = exit.clone();
    ctrlc::set_handler(move || {
        *_ctrlc_exit.lock().expect("exit lock is poisoned!") = true;
    })
    .expect("Failed to set Ctrl-C handler");
    let privileged_token = privileged::check_privilege();
    let state_cycle_handle = std::thread::spawn(move || state.cycle());
    let window_search_handle = std::thread::spawn(move || unsafe {
        window_search_cycle(privileged_token, touch_areas, exit);
    });
    state_cycle_handle.join().unwrap();
    window_search_handle.join().unwrap();
}
