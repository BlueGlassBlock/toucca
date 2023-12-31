use std::ffi::OsString;

use tracing::debug;
use tracing::instrument;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Security::*;
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use wio::wide::FromWide;

unsafe fn obtain_debug_privilege() -> Result<HANDLE> {
    let mut token: HANDLE = Default::default();
    OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &mut token)?;
    let mut tp: TOKEN_PRIVILEGES = Default::default();
    let mut luid: LUID = Default::default();
    LookupPrivilegeValueW(&HSTRING::default(), SE_DEBUG_NAME, &mut luid)?;
    tp.PrivilegeCount = 1;
    tp.Privileges[0].Luid = luid;
    tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
    AdjustTokenPrivileges(
        token,
        FALSE,
        Some(&tp),
        std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
        None,
        None,
    )
    .map_err(|e| {
        if let Err(e2) = CloseHandle(token) {
            eprintln!("Failed to close token handle: {e2}");
        }
        e
    })?;
    Ok(token)
}

#[instrument]
pub(crate) fn check_privilege() -> HANDLE {
    unsafe {
        match obtain_debug_privilege() {
            Err(e) => {
                MessageBoxW(
                HWND(0),
                &HSTRING::from(format!("Failed to obtain debug privilege: {}\nTry running with Administrator rights!", e)),
                &HSTRING::from("Toucca Error"),
                MB_OK | MB_ICONERROR,
            );
                std::process::exit(1);
            }
            Ok(token) => token,
        }
    }
}

// Features of WACCA process:
// Process name: Mercury-Win64-Shipping.exe, Window name: Mercury, location path ends with WindowsNoEditor/Mercury/Binaries/Win64

#[instrument(skip_all)]
unsafe fn iter_proc(snapshot: HANDLE) -> Result<u32> {
    let proc_id;
    let mut proc_info: PROCESSENTRY32W = Default::default();
    proc_info.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    Process32FirstW(snapshot, &mut proc_info)?;
    loop {
        if OsString::from_wide_null(&proc_info.szExeFile)
            .to_string_lossy()
            .ends_with("Mercury-Win64-Shipping.exe")
        {
            debug!("Found Mercury-Win64-Shipping.exe");
            let proc_handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                FALSE,
                proc_info.th32ProcessID,
            )?;
            let mut path_buf: Vec<u16> = vec![0; 1024];
            let mut path_len = path_buf.len() as u32;
            let _r = QueryFullProcessImageNameW(
                proc_handle,
                PROCESS_NAME_WIN32,
                PWSTR(path_buf.as_mut_ptr()),
                &mut path_len,
            );
            CloseHandle(proc_handle)?;
            _r?;
            let path = OsString::from_wide_null(&path_buf[..path_len as usize]);
            debug!("Path: {:?}", path);
            if path
                .to_string_lossy()
                .replace("\\", "/")
                .contains("WindowsNoEditor/Mercury/Binaries/Win64")
            {
                proc_id = proc_info.th32ProcessID;
                debug!("Found Mercury process: {}", proc_id);
                break;
            }
        }
        if Process32NextW(snapshot, &mut proc_info).is_err() {
            proc_id = 0;
            break;
        }
    }
    Ok(proc_id)
}

#[instrument]
pub(crate) unsafe fn find_mercury_proc(token: HANDLE) -> Result<u32> {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
    let res = iter_proc(snapshot);
    CloseHandle(snapshot)?;
    res
}
