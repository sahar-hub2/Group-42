// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

//! Logging subscriber

use std::io::stderr;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::Registry;
use tracing_subscriber::{EnvFilter, fmt};

/// Print to stderr and exit with a non-zero exit code
#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        std::process::exit(1);
    }};
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => {
                fatal!(
                    "Invalid log level `{value}`. Valid levels are: TRACE, DEBUG, INFO, WARN, ERROR"
                );
            }
        }
    }
}

/// Initialize the global logger
pub fn new(log_level: LogLevel) -> WorkerGuard {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level.as_str()))
        .expect("Failed to create log filter");

    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(stderr());

    let logger = Registry::default().with(filter).with(
        fmt::Layer::default()
            .with_writer(non_blocking_stdout)
            .with_file(true)
            .with_line_number(true),
    );

    tracing::subscriber::set_global_default(logger).expect("Failed to initialize logger");

    stdout_guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from("TRACE"), LogLevel::Trace);
        assert_eq!(LogLevel::from("DEBUG"), LogLevel::Debug);
        assert_eq!(LogLevel::from("INFO"), LogLevel::Info);
        assert_eq!(LogLevel::from("WARN"), LogLevel::Warn);
        assert_eq!(LogLevel::from("ERROR"), LogLevel::Error);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }
}
