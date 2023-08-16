use log::{set_logger, set_max_level, Level, LevelFilter, Log, Metadata, Record};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::sync::Once;
use std::thread::current;
use tracing_log::LogTracer;

pub(crate) static LOGGER: Logger = Logger {};
static LOG_TRACER: Lazy<LogTracer> = Lazy::new(LogTracer::new);
static INITLOGGER: Once = Once::new();
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct LogCall {
    pub(crate) level: Level,
    pub(crate) args: String,
    pub(crate) target: String,
    pub(crate) line: Option<u32>,
    pub(crate) file: Option<String>,
    pub(crate) module_path: Option<String>,
}

thread_local!(
    static LOGCALLS: RefCell<Vec<LogCall>> = RefCell::new(Vec::<LogCall>::new());
    static LOGGER_MAX_LEVEL: RefCell<LevelFilter> = RefCell::new(LevelFilter::Off);
);

pub(crate) struct Logger {}

impl Logger {
    pub(crate) fn initialize_test_logger() {
        INITLOGGER.call_once(|| {
            set_logger(&LOGGER).unwrap();
            set_max_level(log::LevelFilter::Trace);
        });
    }

    pub(crate) fn initialize_log_tracer() {
        INITLOGGER.call_once(|| {
            set_logger(&*LOG_TRACER).unwrap();
            set_max_level(log::LevelFilter::Trace);
        });
    }

    pub(crate) fn num_log_calls(&self) -> usize {
        LOGCALLS.with(|log_calls| log_calls.borrow().len())
    }
    pub(crate) fn get_log_call(&self, idx: usize) -> Option<LogCall> {
        LOGCALLS.with(|log_calls| log_calls.borrow().get(idx).cloned())
    }

    pub(crate) fn clear_log_calls(&self) {
        LOGCALLS.with(|log_calls| log_calls.borrow_mut().clear());
    }

    pub(crate) fn test_log_records<F: Fn(&Vec<LogCall>)>(&self, f: F) {
        LOGCALLS.with(|log_calls| f(&log_calls.borrow()));
        self.clear_log_calls();
    }

    pub(crate) fn set_max_level(&self, level: LevelFilter) {
        LOGGER_MAX_LEVEL.with(|max_level| {
            *max_level.borrow_mut() = level;
        });
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        LOGGER_MAX_LEVEL.with(|max_level| metadata.level() <= *max_level.borrow())
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        LOGCALLS.with(|log_calls| {
            log_calls.borrow_mut().push(LogCall {
                level: record.level(),
                args: format!("{}", record.args()),
                target: record.target().to_string(),
                line: record.line(),
                file: match record.file() {
                    None => record.file_static().map(|file| file.to_string()),
                    Some(file) => Some(file.to_string()),
                },
                module_path: match record.module_path() {
                    None => record
                        .module_path_static()
                        .map(|module_path| module_path.to_string()),
                    Some(module_path) => Some(module_path.to_string()),
                },
            })
        });

        println!("Thread {:?} {:?}", current().id(), record);
    }

    fn flush(&self) {}
}
