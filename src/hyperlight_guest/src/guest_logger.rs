use crate::logging::log_message;
use alloc::format;
use log::{LevelFilter, Metadata, Record};

// this is private on purpose so that `log` can only be called though the `log!` macros.
struct GuestLogger {}

pub(crate) fn init_logger(level: LevelFilter) {
    // if this `expect` fails we have no way to recover anyway, so we actually prefer a panic here
    // below temporary guest logger is promoted to static by the compiler.
    log::set_logger(&GuestLogger {}).expect("unable to setup guest logger");
    log::set_max_level(level);
}

impl log::Log for GuestLogger {
    // The various macros like `info!` and `error!` will call the global log::max_level()
    // before calling our `log`. This means that we should log every message we get, because
    // we won't even see the ones that are above the set max level.
    fn enabled(&self, _: &Metadata) -> bool {
        true
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
