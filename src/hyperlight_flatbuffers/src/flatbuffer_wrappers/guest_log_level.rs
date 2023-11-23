use anyhow::{bail, Error, Result};
use log::Level;

use crate::flatbuffers::hyperlight::generated::LogLevel as FbLogLevel;

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Information = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
    None = 6,
}

impl TryFrom<&FbLogLevel> for LogLevel {
    type Error = Error;
    fn try_from(val: &FbLogLevel) -> Result<LogLevel> {
        match *val {
            FbLogLevel::Trace => Ok(LogLevel::Trace),
            FbLogLevel::Debug => Ok(LogLevel::Debug),
            FbLogLevel::Information => Ok(LogLevel::Information),
            FbLogLevel::Warning => Ok(LogLevel::Warning),
            FbLogLevel::Error => Ok(LogLevel::Error),
            FbLogLevel::Critical => Ok(LogLevel::Critical),
            FbLogLevel::None => Ok(LogLevel::None),
            _ => {
                bail!("Unsupported Flatbuffers log level: {:?}", val);
            }
        }
    }
}

impl From<&LogLevel> for FbLogLevel {
    fn from(val: &LogLevel) -> FbLogLevel {
        match val {
            LogLevel::Critical => FbLogLevel::Critical,
            LogLevel::Debug => FbLogLevel::Debug,
            LogLevel::Error => FbLogLevel::Error,
            LogLevel::Information => FbLogLevel::Information,
            LogLevel::None => FbLogLevel::None,
            LogLevel::Trace => FbLogLevel::Trace,
            LogLevel::Warning => FbLogLevel::Warning,
        }
    }
}

impl From<&LogLevel> for Level {
    fn from(val: &LogLevel) -> Level {
        match val {
            LogLevel::Trace => Level::Trace,
            LogLevel::Debug => Level::Debug,
            LogLevel::Information => Level::Info,
            LogLevel::Warning => Level::Warn,
            LogLevel::Error => Level::Error,
            LogLevel::Critical => Level::Error,
            // If the log level is None then we will log as trace
            LogLevel::None => Level::Trace,
        }
    }
}
