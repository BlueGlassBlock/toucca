use serial::SerialPort;
use serialport as serial;
use std::{
    collections::HashSet,
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tracing::{debug, error, info};

mod constant {
    pub const GET_SYNC_BOARD_VER: u8 = 0xA0;
    pub const NEXT_READ: u8 = 0x72;
    pub const GET_UNIT_BOARD_VER: u8 = 0xA8;
    pub const MYSTERY_1: u8 = 0xA2;
    pub const MYSTERY_2: u8 = 0x94;
    pub const START_AUTO_SCAN: u8 = 0xC9;
    pub const BEGIN_WRITE: u8 = 0x77;
    pub const NEXT_WRITE: u8 = 0x20;
    pub const BAD_IN_BYTE: u8 = 0x9A;
    // pub const DATA_160: [u8; 8] = [160, 49, 57, 48, 53, 50, 51, 44];
    pub const DATA_162: [u8; 3] = [162, 63, 29];
    pub const DATA_148: [u8; 3] = [148, 0, 20];
    pub const DATA_201: [u8; 3] = [201, 0, 73];
    pub const SYNC_BOARD_VER: &str = "190523";
    pub const UNIT_BOARD_VER: &str = "190514";
    pub const READ_1: &str =
        "    0    0    1    2    3    4    5   15   15   15   15   15   15   11   11   11";
    pub const READ_2: &str =
        "   11   11   11  128  103  103  115  138  127  103  105  111  126  113   95  100";
    pub const READ_3: &str =
        "  101  115   98   86   76   67   68   48  117    0   82  154    0    6   35    4";
    pub const UNIT_R: u8 = 118;
    pub const UNIT_L: u8 = 104;
}

mod pack {
    pub type Pack = [u8; 36];

    pub fn checksum(pack: &[u8]) -> u8 {
        let mut val = 0;
        for byte in pack {
            val ^= byte;
        }
        val
    }

    pub fn set(pack: &mut Pack, index: usize, value: bool) {
        let byte_index = index / 8;
        let bit_index = index % 8;
        let byte = &mut pack[byte_index];
        if value {
            *byte |= 1 << bit_index;
        } else {
            *byte &= !(1 << bit_index);
        }
    }

    pub fn prepare(mut pack: Pack) -> Pack {
        pack[0] = 129;
        pack[34] += 1;
        pack[35] = 128;
        pack[35] = super::pack::checksum(&pack);
        if pack[34] > 127 {
            pack[34] = 0;
        }
        pack
    }
}

struct TouccaState {
    ports: [serial::COMPort; 2],
    startup_complete: bool,
    touch_areas: Arc<Mutex<HashSet<usize>>>,
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
        Self {
            ports: [com_l, com_r],
            startup_complete: false,
            touch_areas: Default::default(),
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

    fn cycle(&mut self) -> ! {
        use std::thread::sleep;
        use std::time::Duration;
        loop {
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

const INFO: &str = "Toucca by BlueGlassBlock
This program is free and licensed under the GNU Public License v3.0
Visit https://github.com/BlueGlassBlock/toucca for more information.";

pub fn main() {
    println!("{}", INFO);
    let mut state = TouccaState::new();
    let touch_areas = state.touch_areas.clone();
    let thread_handle = std::thread::spawn(move || state.cycle());
    
}
