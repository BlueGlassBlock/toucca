use std::io::Write;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, Layer, util::SubscriberInitExt};

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

const LEVEL_FILTER: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::DEBUG
} else {
    LevelFilter::INFO
};

pub fn init_log() {
    std::panic::set_hook(Box::new(move |info| {
        WinDebugWriter
            .write_fmt(format_args!("Toucca panicked: {}", info))
            .unwrap();
    }));

    let file_appender = tracing_appender::rolling::daily(".", "toucca.log");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(WinDebugWriter::new)
                .without_time()
                .with_filter(LEVEL_FILTER),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_filter(LEVEL_FILTER),
        )
        .init();
}
