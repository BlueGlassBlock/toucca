use std::io::Write;

pub struct WinDebugWriter;

impl Write for WinDebugWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            ::windows::Win32::System::Diagnostics::Debug::OutputDebugStringW(
                &::windows::core::HSTRING::from(String::from_utf8_lossy(buf).to_string()),
            );
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl WinDebugWriter {
    pub fn new() -> Self {
        Self
    }
}

pub fn init_log() {
    std::panic::set_hook(Box::new(move |info| {
        WinDebugWriter
            .write_fmt(format_args!("Toucca panicked: {}", info))
            .unwrap();
    }));

    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(WinDebugWriter::new)
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}
