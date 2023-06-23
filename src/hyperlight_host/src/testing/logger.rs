use std::{cell::RefCell, sync::Mutex};

use log::{Level, Log, Metadata, Record};

pub(crate) static LOGGER: Logger = Logger {
    log_calls: Mutex::new(RefCell::new(Vec::new())),
};

#[derive(Clone, Eq, PartialEq)]
#[readonly::make]
pub(crate) struct LogCall {
    pub level: Level,
}

pub(crate) struct Logger {
    log_calls: Mutex<RefCell<Vec<LogCall>>>,
}

impl Logger {
    #[cfg(test)]
    pub(crate) fn num_log_calls(&self) -> usize {
        let unlocked_log_calls = self.log_calls.lock().unwrap();
        let log_calls = unlocked_log_calls.borrow();
        log_calls.len()
    }
    pub(crate) fn get_log_call(&self, idx: usize) -> Option<LogCall> {
        let unlocked_log_calls = self.log_calls.lock().unwrap();
        let log_calls = unlocked_log_calls.borrow();
        log_calls.get(idx).cloned()
    }
}
impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        let mut unlocked_log_calls = self.log_calls.lock().unwrap();
        let v = unlocked_log_calls.get_mut();
        v.push(LogCall {
            level: record.level(),
        });
    }

    fn flush(&self) {}
}
