use crate::flatbuffers::hyperlight::generated::LogLevel as GenLogLevel;
use anyhow::{bail, Result};

#[repr(u8)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) enum LogLevel {
    Trace = 0,
    Debug = 1,
    Information = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
    None = 6,
}

impl From<LogLevel> for u8 {
    fn from(val: LogLevel) -> u8 {
        // this cast is legal because the LogLevel enum is
        // defined as #[repr(u8)]
        val as u8
    }
}

impl TryFrom<GenLogLevel> for LogLevel {
    type Error = anyhow::Error;
    fn try_from(val: GenLogLevel) -> Result<LogLevel> {
        match val {
            GenLogLevel::Trace => Ok(LogLevel::Trace),
            GenLogLevel::Debug => Ok(LogLevel::Debug),
            GenLogLevel::Information => Ok(LogLevel::Information),
            GenLogLevel::Warning => Ok(LogLevel::Warning),
            GenLogLevel::Error => Ok(LogLevel::Error),
            GenLogLevel::Critical => Ok(LogLevel::Critical),
            GenLogLevel::None => Ok(LogLevel::None),
            _ => bail!("Unsupported Flatbuffers log level: {:?}", val),
        }
    }
}

impl From<&LogLevel> for GenLogLevel {
    fn from(val: &LogLevel) -> GenLogLevel {
        match val {
            LogLevel::Critical => GenLogLevel::Critical,
            LogLevel::Debug => GenLogLevel::Debug,
            LogLevel::Error => GenLogLevel::Error,
            LogLevel::Information => GenLogLevel::Information,
            LogLevel::None => GenLogLevel::None,
            LogLevel::Trace => GenLogLevel::Trace,
            LogLevel::Warning => GenLogLevel::Warning,
        }
    }
}

impl From<LogLevel> for GenLogLevel {
    fn from(val: LogLevel) -> GenLogLevel {
        GenLogLevel::from(&val)
    }
}
