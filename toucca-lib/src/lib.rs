use tracing::debug;

pub mod serial;
pub mod config;
pub mod window;

use config::TouccaConfig;
use windows::core::*;

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

pub fn is_mercury_process() -> Result<bool> {
    use windows::Win32::System::Threading::*;
    use std::ffi::OsString;
    use wio::wide::FromWide;
    unsafe {
        let proc_id = GetCurrentProcessId();
        // check process.name == "Mercury-Win64-Shipping.exe"
        let mut proc_name = [0u16; 1024];
        let mut proc_name_len = proc_name.len() as u32;
        let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, proc_id)?;
        QueryFullProcessImageNameW(proc_handle, PROCESS_NAME_WIN32, PWSTR(proc_name.as_mut_ptr()), &mut proc_name_len)?;
        let proc_name = OsString::from_wide_null(&proc_name[..proc_name_len as usize]);
        Ok(proc_name.to_string_lossy().contains("Mercury-Win64-Shipping.exe"))
    }
}