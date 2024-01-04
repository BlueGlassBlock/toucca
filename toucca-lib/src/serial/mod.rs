mod constant;
mod pack;

use std::collections::HashSet;
use std::io::{Read, Write};
use std::thread::JoinHandle;

use serial::SerialPort;
use serialport as serial;
use tracing::{debug, error, info, instrument};

use super::window::*;
use super::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;

struct TouccaState {
    ports: [serial::COMPort; 2],
    startup_complete: bool,
    touch_areas: HashSet<usize>,
    hwnd: Option<HWND>,
}

// FIXME: section is sometimes off

impl TouccaState {
    #[instrument]
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
            ports: [com_r, com_l],
            startup_complete: false,
            touch_areas: Default::default(),
            hwnd: None,
        }
    }

    #[instrument(skip_all, fields(side = side))]
    fn make_resp(side: usize, data: &[u8]) -> (Option<bool>, Option<Vec<u8>>) {
        let mut startup_complete = None;
        let mut resp_data = None;
        let head = data[0];
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
                match data.get(3).unwrap_or(&0) {
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
            let mut buf = vec![0; to_read];
            port.read_exact(&mut buf)?;
            let (startup_complete, resp) = Self::make_resp(side, &buf);
            if let Some(startup_complete) = startup_complete {
                if self.startup_complete != startup_complete {
                    info!("Update startup complete status: {}", startup_complete);
                }
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
        for &area in self.touch_areas.iter() {
            let side = if area >= 120 { 0 } else { 1 };
            let mut index = area % 120;
            index += index / 5 * 3 + 8;
            pack::set(&mut packs[side], index, true);
        }
        for (side, port) in self.ports.iter_mut().enumerate() {
            let pack = pack::prepare(packs[side]);
            port.write_all(&pack)?;
        }
        Ok(())
    }

    #[instrument(skip_all)]
    unsafe fn cycle(&mut self) {
        use std::thread::sleep;
        use std::time::Duration;
        loop {
            // check whether the window is still alive
            if let Some(handle) = self.hwnd {
                if !IsWindow(handle).as_bool() {
                    self.hwnd = None;
                }
            }
            if self.hwnd.is_none() {
                self.hwnd = get_window_handle(GetCurrentProcessId());
                if let Some(hwnd) = self.hwnd {
                    hook_wnd_proc(hwnd);
                }
            }
            self.touch_areas = window::get_active_areas();
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

pub fn start_serial() -> JoinHandle<()> {
    let mut state = TouccaState::new();
    std::thread::spawn(move || unsafe { state.cycle() })
}
