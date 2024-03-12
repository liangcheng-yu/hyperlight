use alloc::format;
use log::{LevelFilter, Metadata, Record};

use crate::logging::log_message;

pub(crate) struct GuestLogger {
    max_level: LevelFilter,
}
impl GuestLogger {
    pub(crate) fn set_max_level(max_level: LevelFilter) {
        unsafe {
            LOGGER.max_level = max_level;
        }
    }
}

impl log::Log for GuestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            log_message(
                record.metadata().level().into(),
                format!("{}", record.args()).as_str(),
                record.module_path().unwrap_or("Unknown"),
                record.target(),
                record.file().unwrap_or("Unknown"),
                record.line().unwrap_or(0),
            );
        }
    }

    fn flush(&self) {}
}

pub(crate) static mut LOGGER: GuestLogger = GuestLogger {
    max_level: LevelFilter::Off,
};
