use tracing::debug;

pub mod config;
pub mod window;

use config::TouccaConfig;
use windows::core::HSTRING;

pub(crate) fn lo_word(wparam: windows::Win32::Foundation::WPARAM) -> u16 {
    (wparam.0 & 0xffff) as u16
}

pub static mut CONFIG: TouccaConfig = TouccaConfig::default();

pub fn load_segatools_config() {
    debug!("Reading config from segatools.ini");
    unsafe {
        CONFIG = TouccaConfig::load(&HSTRING::from(".\\segatools.ini"));
    }
}
