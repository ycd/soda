use std::borrow::{Borrow, BorrowMut};

use fern::Dispatch;
use log::{debug, error, info, trace, warn};

use pyo3::prelude::*;
use pyo3::types::{PyLong, PyUnicode};

#[pymodule]
fn soda(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Soda>()?;
    Ok(())
}

/// Until https://github.com/PyO3/pyo3/issues/417
/// gets merged, we cannot bind rust enums or constants
/// as a part of module
pub enum Level {
    NOTSET,
    DEBUG,
    INFO,
    WARNING,
    ERROR,
    CRITICAL,
}

static dateFormat: &'static str = "[%Y-%m-%d][%H:%M:%S]";

#[pyclass(dict, subclass)]
pub struct Soda {
    pub level: Level,

    pub format: String,
    // pub verbosity: u64
    // pub handlers: Vec<PyFunction>, TODO(ycd) support custom handlers
}

#[pymethods]
impl Soda {
    #[new]
    #[args(verbosity = "0")]
    fn new(verbosity: u64) -> Soda {
        // Create at Python runtime to make this logger globally accessable.
        let mut base_config = fern::Dispatch::new();
        base_config = match verbosity {
            0 => base_config
                .level(log::LevelFilter::Info)
                .level_for("overly-verbose-target", log::LevelFilter::Warn),
            1 => base_config
                .level(log::LevelFilter::Debug)
                .level_for("overly-verbose-target", log::LevelFilter::Info),
            2 => base_config.level(log::LevelFilter::Debug),
            _3_or_more => base_config.level(log::LevelFilter::Trace),
        };

        Soda {
            level: Level::NOTSET,
            format: String::new(),
        }
    }

    fn setFormat(&mut self, format: &PyUnicode) {
        let format: Result<&str, PyErr> = format.to_str();

        if let Ok(format) = format {
            self.format = format.to_string();
        }
    }

    // #[args(path=dateFormat)]
    /// Setup the configuration for the file
    fn fileConfig(&mut self, path: &PyUnicode) {
        let path: String = match path.to_str() {
            Ok(p) => p.to_string(),
            Err(e) => {
                println!("An error occured while reading the path {}", e);
                format!("{}.log", chrono::Utc::now().format("%Y-%m-%d:%H:%M:%S"))
            }
        };

        let file_config = &fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .chain(fern::log_file(path).unwrap())
            .apply();
    }

    fn basicConfig(&mut self, dtFormat: &PyUnicode) {
        let dtFormat: String = match dtFormat.to_str() {
            Ok(fmt) => fmt.to_string(),
            Err(e) => {
                println!(
                    "An error occured while reading the format {}, using the default format",
                    e
                );
                String::from(dateFormat)
            }
        };

        let stdout_config = fern::Dispatch::new()
            .format(move |out, message, record| {
                // special format for debug messages coming from our own crate.
                if record.level() > log::LevelFilter::Info && record.target() == "" {
                    out.finish(format_args!(
                        "---\nDEBUG: {}: {}\n---",
                        chrono::Local::now().format(dtFormat.as_str()),
                        message
                    ))
                } else {
                    out.finish(format_args!(
                        "[{}][{}][{}] {}",
                        chrono::Local::now().format(dtFormat.as_str()),
                        record.target(),
                        record.level(),
                        message
                    ))
                }
            })
            .chain(std::io::stdout())
            .apply();
    }

    fn info(&self, message: &PyUnicode) {
        let message = match message.to_str() {
            Ok(msg) => msg,
            _ => return,
        };

        info!("{}", message);
    }

    fn warn(&mut self, message: &PyUnicode) {
        let message = match message.to_str() {
            Ok(msg) => msg,
            _ => return,
        };

        warn!("{}", message);
    }

    fn debug(&mut self, message: &PyUnicode) {
        let message = match message.to_str() {
            Ok(msg) => msg,
            _ => return,
        };

        debug!("{}", message);
    }

    fn trace(&mut self, message: &PyUnicode) {
        let message = match message.to_str() {
            Ok(msg) => msg,
            _ => return,
        };

        trace!("{}", message);
    }

    fn error(&mut self, message: &PyUnicode) {
        let message = match message.to_str() {
            Ok(msg) => msg,
            _ => return,
        };

        error!("{}", message);
    }

    pub fn setLevel(&mut self, level: &PyUnicode) {
        let level: Result<&str, PyErr> = level.to_str();

        match level {
            Ok("DEBUG") => self.level = Level::DEBUG,
            Ok("INFO") => self.level = Level::INFO,
            Ok("WARNING") => self.level = Level::WARNING,
            Err(err) => {
                println!("An error occured {}", err);
            }
            _ => {
                println!("Found none, setting default value to 'DEBUG'");
                self.level = Level::DEBUG
            }
        }
    }
}

fn setup_logging(verbosity: u64) -> Result<(), fern::InitError> {
    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => {
            // Let's say we depend on something which whose "info" level messages are too
            // verbose to include in end-user output. If we don't need them,
            // let's not include them.
            base_config
                .level(log::LevelFilter::Info)
                .level_for("overly-verbose-target", log::LevelFilter::Warn)
        }
        1 => base_config
            .level(log::LevelFilter::Debug)
            .level_for("overly-verbose-target", log::LevelFilter::Info),
        2 => base_config.level(log::LevelFilter::Debug),
        _3_or_more => base_config.level(log::LevelFilter::Trace),
    };

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file("program.log")?);

    let stdout_config = fern::Dispatch::new()
        .format(|out, message, record| {
            // special format for debug messages coming from our own crate.
            if record.level() > log::LevelFilter::Info && record.target() == "" {
                out.finish(format_args!(
                    "---\nDEBUG: {}: {}\n---",
                    chrono::Local::now().format("%H:%M:%S"),
                    message
                ))
            } else {
                out.finish(format_args!(
                    "[{}][{}][{}] {}",
                    chrono::Local::now().format("%H:%M"),
                    record.target(),
                    record.level(),
                    message
                ))
            }
        })
        .chain(std::io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}
