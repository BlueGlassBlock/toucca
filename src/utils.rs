use windows::Win32::Foundation::WPARAM;

#[macro_export]
macro_rules! dprintln {
    (dbg, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            dprintln!($($arg)*);
        }
    };
    ($($arg:tt)*) => {
        #[allow(unused_unsafe)]
        unsafe {
            ::windows::Win32::System::Diagnostics::Debug::OutputDebugStringW(&::windows::core::HSTRING::from(format_args!($($arg)*).to_string()));
            ::windows::Win32::System::Diagnostics::Debug::OutputDebugStringW(&::windows::core::HSTRING::from("\n"));
        }
    }
}

pub trait DebugUnwrap<T> {
    fn dbg_unwrap(self) -> T;
}

impl<T, E> DebugUnwrap<T> for Result<T, E>
where
    E: std::fmt::Debug,
{
    fn dbg_unwrap(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                panic!("Error: {:?}", e);
            },
        }
    }
}

#[allow(unused)]
pub fn lo_word(wparam: WPARAM) -> u16 {
    (wparam.0 & 0xffff) as u16
}

#[allow(unused)]
pub fn hi_word(wparam: WPARAM) -> u16 {
    ((wparam.0 >> 16) & 0xffff) as u16
}
