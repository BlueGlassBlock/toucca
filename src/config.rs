use std::collections::HashMap;
use std::ops::Add;
use std::sync::Mutex;

use windows::core::*;
use windows::Win32::System::WindowsProgramming::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use crate::dprintln;

pub struct TouccaTouchConfig {
    pub divisions: usize,
    pub radius_compensation: i32,
    pub pointer_radius: u32,
    pub mode: TouccaMode, // 0 - Absolute, 1 - Relative
}

#[derive(Debug)]
pub struct TouccaRelativeConfig {
    pub start: usize,
    pub threshold: usize,
    pub map_lock: Mutex<HashMap<u32, (usize, usize)>>, // pointer id -> (physical ring, virtual ring)
}

#[derive(Debug)]
pub enum TouccaMode {
    Absolute([(usize, usize); 4]),
    Relative(TouccaRelativeConfig),
}

impl TouccaMode {
    fn flip_left_ring(section: usize) -> usize {
        if section >= 30 {30 + 59 - section} else { section }
    }
    pub fn expand_section_with_radius(radius: u32, section: usize) -> Vec<usize> {
        let mut res = vec![section];
        let section = 60 + TouccaMode::flip_left_ring(section);
        for to_add in 1..radius as usize {
            let left = (section + to_add) % 60;
            let right = (section - to_add) % 60;
            res.push(TouccaMode::flip_left_ring(left));
            res.push(TouccaMode::flip_left_ring(right));
        }
        res
    }

    fn map_section_and_ring(section: usize, ring: usize) -> usize {
        ring * 30 + section % 30 + if section >= 30 { 120 } else { 0 }
    }

    fn convert_single(&self, ptr_id: u32, section: usize, ring: usize) -> Vec<usize> {
        match self {
            Self::Relative(cfg) => {
                let TouccaRelativeConfig {
                    start,
                    threshold,
                    map_lock,
                } = &cfg;
                let mut guard = map_lock.lock().unwrap();
                if let Some((p_ring, v_ring)) = guard.get(&ptr_id) {
                    let diff = (ring as i64 - *p_ring as i64) / *threshold as i64;
                    let v_ring = (*v_ring as i64).add(diff).clamp(0, 3) as usize;
                    guard.insert(ptr_id, (ring, v_ring));
                    vec![Self::map_section_and_ring(section, v_ring)]
                } else {
                    guard.insert(ptr_id, (ring, *start));
                    vec![Self::map_section_and_ring(section, *start)]
                }
            }
            Self::Absolute(ranges) => {
                let mut res = vec![];
                for (i, (st, end)) in ranges.iter().enumerate() {
                    if *st <= ring && ring <= *end {
                        res.push(Self::map_section_and_ring(i, ring));
                        dprintln!("{} {} -> {}", section, i, res.last().unwrap());
                    }
                }
                res
            }
        }
    }

    pub fn to_cells(&self, ptr_id: u32, section: usize, ring: usize, radius: u32) -> Vec<usize> {
        let mut res = vec![];
        for section in Self::expand_section_with_radius(radius, section) {
            res.extend(self.convert_single(ptr_id, section, ring));
        }
        res
    }
}

pub struct TouccaConfig {
    pub vk_test: i32,
    pub vk_service: i32,
    pub vk_coin: i32,
    pub vk_vol_up: i32,
    pub vk_vol_down: i32,
    pub vk_cell: [i32; 240],
    pub touch: TouccaTouchConfig,
}

impl TouccaTouchConfig {
    pub unsafe fn load(filename: &HSTRING) -> Self {
        let divisions = GetPrivateProfileIntW(h!("touch"), h!("divisions"), 8, filename) as usize;
        if !(4..=20).contains(&divisions) {
            panic!("Invalid touch divisions");
        }
        let radius_compensation =
            GetPrivateProfileIntW(h!("touch"), h!("radius_compensation"), 0, filename);
        let pointer_radius = GetPrivateProfileIntW(h!("touch"), h!("pointer_radius"), 1, filename);
        if !(1..=10).contains(&pointer_radius) {
            panic!("Invalid finger radius: {}", pointer_radius);
        }
        let pointer_radius = pointer_radius as u32;
        let mode = match GetPrivateProfileIntW(h!("touch"), h!("mode"), 0, filename) as usize {
            0 => {
                let mut ranges = [(0, 0); 4];
                for (i, range) in ranges.iter_mut().enumerate() {
                    *range = (divisions - 4 + i, divisions - 4 + i);
                    let single = GetPrivateProfileIntW(
                        h!("touch"),
                        &HSTRING::from(format!("ring{}", i)),
                        -1,
                        filename,
                    );
                    let start = GetPrivateProfileIntW(
                        h!("touch"),
                        &HSTRING::from(format!("ring{}_start", i)),
                        -1,
                        filename,
                    );
                    let end = GetPrivateProfileIntW(
                        h!("touch"),
                        &HSTRING::from(format!("ring{}_end", i)),
                        -1,
                        filename,
                    );
                    if single != -1 {
                        *range = (single as usize, single as usize);
                    }
                    if start != -1 {
                        range.0 = start as usize;
                    }
                    if end != -1 {
                        range.1 = end as usize;
                    }
                    // check range
                    if range.0 > range.1 || range.1 >= divisions {
                        panic!("Invalid touch range: {}-{}", range.0, range.1);
                    }
                    dprintln!("Set ring {} touch range: {} - {}", i, range.0, range.1);
                }
                TouccaMode::Absolute(ranges)
            }
            1 => {
                let start =
                    GetPrivateProfileIntW(h!("touch"), h!("relative_start"), 1, filename) as usize;
                // check start in 0-3
                if start > 3 {
                    panic!("Invalid relative touch start position");
                }
                let threshold =
                    GetPrivateProfileIntW(h!("touch"), h!("relative_threshold"), 1, filename)
                        as usize;
                TouccaMode::Relative(TouccaRelativeConfig {
                    start,
                    threshold,
                    map_lock: Mutex::new(HashMap::new()),
                })
            }
            _ => {
                panic!("Invalid touch mode");
            }
        };
        dprintln!("Using touch mode: {:?}", mode);
        Self { divisions, pointer_radius, radius_compensation, mode }
    }
}

impl TouccaConfig {
    pub unsafe fn load(filename: &HSTRING) -> Self {
        let mut cells = [0; 240];
        for i in 0..240 {
            cells[i] = GetPrivateProfileIntW(
                h!("touch"),
                &HSTRING::from(format!("cell{}", i + 1)),
                MERCURY_IO_DEFAULT_CELLS[i].0.into(),
                filename,
            );
        }
        Self {
            vk_test: GetPrivateProfileIntW(h!("io4"), h!("test"), VK_INSERT.0.into(), filename),
            vk_service: GetPrivateProfileIntW(h!("io4"), h!("test"), VK_DELETE.0.into(), filename),
            vk_coin: GetPrivateProfileIntW(h!("io4"), h!("coin"), VK_HOME.0.into(), filename),
            vk_vol_up: GetPrivateProfileIntW(h!("io4"), h!("volup"), VK_UP.0.into(), filename),
            vk_vol_down: GetPrivateProfileIntW(
                h!("io4"),
                h!("voldown"),
                VK_DOWN.0.into(),
                filename,
            ),
            vk_cell: cells,
            touch: TouccaTouchConfig::load(filename),
        }
    }
    pub const fn default() -> Self {
        Self {
            vk_test: VK_INSERT.0 as i32,
            vk_service: VK_DELETE.0 as i32,
            vk_coin: VK_HOME.0 as i32,
            vk_vol_up: VK_UP.0 as i32,
            vk_vol_down: VK_DOWN.0 as i32,
            vk_cell: [0; 240],
            touch: TouccaTouchConfig {
                divisions: 8,
                pointer_radius: 1,
                radius_compensation: 30,
                mode: TouccaMode::Absolute([(0, 0); 4]),
            },
        }
    }
}

// from segatools
const MERCURY_IO_DEFAULT_CELLS: [VIRTUAL_KEY; 240] = [
    // 1234567890
    VK_1, VK_1, VK_1, VK_2, VK_2, VK_2, VK_3, VK_3, VK_3, VK_4, VK_4, VK_4, VK_5, VK_5, VK_5, VK_6,
    VK_6, VK_6, VK_7, VK_7, VK_7, VK_8, VK_8, VK_8, VK_9, VK_9, VK_9, VK_0, VK_0,
    VK_0, // 0 - 29 (ring 0, section 0 - 29)
    VK_1, VK_1, VK_1, VK_2, VK_2, VK_2, VK_3, VK_3, VK_3, VK_4, VK_4, VK_4, VK_5, VK_5, VK_5, VK_6,
    VK_6, VK_6, VK_7, VK_7, VK_7, VK_8, VK_8, VK_8, VK_9, VK_9, VK_9, VK_0, VK_0,
    VK_0, // 30 - 59 (ring 1, section 0 - 29)
    // QWERTYUIOP
    VK_Q, VK_Q, VK_Q, VK_W, VK_W, VK_W, VK_E, VK_E, VK_E, VK_R, VK_R, VK_R, VK_T, VK_T, VK_T, VK_Y,
    VK_Y, VK_Y, VK_U, VK_U, VK_U, VK_I, VK_I, VK_I, VK_O, VK_O, VK_O, VK_P, VK_P,
    VK_P, // 60 - 89 (ring 2, section 0 - 29)
    VK_Q, VK_Q, VK_Q, VK_W, VK_W, VK_W, VK_E, VK_E, VK_E, VK_R, VK_R, VK_R, VK_T, VK_T, VK_T, VK_Y,
    VK_Y, VK_Y, VK_U, VK_U, VK_U, VK_I, VK_I, VK_I, VK_O, VK_O, VK_O, VK_P, VK_P,
    VK_P, // 90 - 119 (ring 3, section 0 - 29)
    // ASDFGHJKL;
    VK_A, VK_A, VK_A, VK_S, VK_S, VK_S, VK_D, VK_D, VK_D, VK_F, VK_F, VK_F, VK_G, VK_G, VK_G, VK_H,
    VK_H, VK_H, VK_J, VK_J, VK_J, VK_K, VK_K, VK_K, VK_L, VK_L, VK_L, VK_OEM_1, VK_OEM_1,
    VK_OEM_1, // 120 - 149 (ring 0, section 30 - 59)
    VK_A, VK_A, VK_A, VK_S, VK_S, VK_S, VK_D, VK_D, VK_D, VK_F, VK_F, VK_F, VK_G, VK_G, VK_G, VK_H,
    VK_H, VK_H, VK_J, VK_J, VK_J, VK_K, VK_K, VK_K, VK_L, VK_L, VK_L, VK_OEM_1, VK_OEM_1,
    VK_OEM_1, // 150 - 179 (ring 1, section 30 - 59)
    // ZXCVBNM,./
    VK_Z, VK_Z, VK_Z, VK_X, VK_X, VK_X, VK_C, VK_C, VK_C, VK_V, VK_V, VK_V, VK_B, VK_B, VK_B, VK_N,
    VK_N, VK_N, VK_M, VK_M, VK_M, VK_OEM_COMMA, VK_OEM_COMMA, VK_OEM_COMMA, VK_OEM_PERIOD,
    VK_OEM_PERIOD, VK_OEM_PERIOD, VK_OEM_2, VK_OEM_2,
    VK_OEM_2, // 180 - 209 (ring 2, section 30 - 59)
    VK_Z, VK_Z, VK_Z, VK_X, VK_X, VK_X, VK_C, VK_C, VK_C, VK_V, VK_V, VK_V, VK_B, VK_B, VK_B, VK_N,
    VK_N, VK_N, VK_M, VK_M, VK_M, VK_OEM_COMMA, VK_OEM_COMMA, VK_OEM_COMMA, VK_OEM_PERIOD,
    VK_OEM_PERIOD, VK_OEM_PERIOD, VK_OEM_2, VK_OEM_2,
    VK_OEM_2, // 210 - 239 (ring 3, section 30 - 59)
];
