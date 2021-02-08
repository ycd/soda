use std::{
    borrow::{Borrow, BorrowMut},
    fs::File,
    io::{ErrorKind, Write},
};

use std::fs::OpenOptions;
use std::io::prelude::*;

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
    pub handlers: Handlers,
}

#[pyclass(dict, subclass)]
pub struct Handlers {
    FileHandler: FileLogger,
}

#[pymethods]
impl Handlers {
    #[new]
    #[args(json = false, file = false)]
    fn new(json: bool, file: bool) -> Handlers {
        Handlers {
            FileHandler: FileLogger::new(),
        }
    }
}

#[pymethods]
impl Soda {
    #[new]
    #[args(verbosity = "0")]
    fn new(verbosity: u64) -> Soda {
        // Create at Python runtime to make this logger globally accessable.
        let mut base_config = fern::Dispatch::new();

        base_config = match verbosity {
            0 => base_config.level(log::LevelFilter::Info),
            1 => base_config.level(log::LevelFilter::Debug),
            2 => base_config.level(log::LevelFilter::Warn),
            _3_or_more => base_config.level(log::LevelFilter::Trace),
        };

        Soda {
            level: Level::NOTSET,
            format: String::new(),
            handlers: Handlers::new(false, false),
        }
    }

    fn setFormat(&mut self, format: &PyUnicode) {
        let format: Result<&str, PyErr> = format.to_str();

        if let Ok(format) = format {
            self.format = format.to_string();
        }
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

        let mut config = fern::Dispatch::new()
            .format(move |out, message, record| {
                // special format for debug messages coming from our own crate.
                if record.level() > log::LevelFilter::Info && record.target() == "soda" {
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

        self.callback(message);
    }

    fn addFileHandler(&mut self, path: String) {
        let f = File::open(&path);

        let _: File = match f {
            Ok(file) => file,
            Err(error) => match error.kind() {
                ErrorKind::NotFound => match File::create(&path) {
                    Ok(fc) => fc,
                    Err(e) => panic!("Problem creating the file: {:?}", e),
                },
                _ => panic!("an error occured {}", error),
            },
        };

        self.handlers.FileHandler.enabled = true;
        self.handlers.FileHandler.path = path;
    }

    fn callback(&self, message: &str) {
        match self.handlers.FileHandler.enabled {
            true => self.handlers.FileHandler.logger(message),
            false => (),
        };

        // TODO(ycd): enable json logging with extra crate.
        // match self.handlers.JsonHandler {
        //     // true => jsonLogger(message),
        //     true => (),
        //     false => (),
        // };
    }

    fn warning(&mut self, message: &PyUnicode) {
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

    pub fn setLevel(&mut self, verbosity: u8) {
        match verbosity {
            1 => self.level = Level::DEBUG,
            2 => self.level = Level::INFO,
            3 => self.level = Level::WARNING,
            _ => {
                println!("Found none, setting default value to 'DEBUG'");
                self.level = Level::DEBUG
            }
        }
    }
}

// fn fileLogger(message: &str) {
//     let mut file = OpenOptions::new()
//         .write(true)
//         .append(true)
//         .open(&self.path)
//         .unwrap();

//     if let Err(e) = writeln!(file, "{}", self.format(message)) {
//         eprintln!("Couldn't write to file: {}", e);
//     }

//     let f = File::open(&self.path);

//     let f: File = match f {
//         Ok(file) => file,
//         Err(error) => match error.kind() {
//             ErrorKind::NotFound => match File::create(&self.path) {
//                 Ok(fc) => fc,
//                 Err(e) => panic!("Problem creating the file: {:?}", e),
//             },
//             _ => panic!("an error occured {}", error),
//         },
//     };
// }

// trait Logger {
//     fn logger(message: &str);
// }

struct FileLogger {
    enabled: bool,
    path: String,
}

impl FileLogger {
    fn new() -> FileLogger {
        FileLogger {
            enabled: false,
            path: String::from("default.log"),
        }
    }

    fn logger(&self, message: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.path)
            .unwrap();

        if let Err(e) = writeln!(file, "{}", message) {
            eprintln!("Couldn't write to file: {}", e);
        }

        // let f = File::open(&self.path);

        // let f: File = match f {
        //     Ok(file) => file,
        //     Err(error) => match error.kind() {
        //         ErrorKind::NotFound => match File::create(&self.path) {
        //             Ok(fc) => fc,
        //             Err(e) => panic!("Problem creating the file: {:?}", e),
        //         },
        //         _ => panic!("an error occured {}", error),
        //     },
        // };
    }

    fn format(&self, message: &str) -> String {
        format!("{}", message)
    }
}

// impl Soda {
//     // fn _addConfig(&mut self, config: &fern::Dispatch) {

//     // }
// }

// fn setup_logging(verbosity: u64) -> Result<(), fern::InitError> {
//     let mut base_config = fern::Dispatch::new();

//     base_config = match verbosity {
//         0 => base_config
//             .level(log::LevelFilter::Info)
//             .level_for("overly-verbose-target", log::LevelFilter::Warn),
//         1 => base_config
//             .level(log::LevelFilter::Debug)
//             .level_for("overly-verbose-target", log::LevelFilter::Info),
//         2 => base_config.level(log::LevelFilter::Debug),
//         _3_or_more => base_config.level(log::LevelFilter::Trace),
//     };

//     // Separate file config so we can include year, month and day in file logs

//     let file_config = fern::Dispatch::new()
//         .format(|out, message, record| {
//             out.finish(format_args!(
//                 "{}[{}][{}] {}",
//                 chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
//                 record.target(),
//                 record.level(),
//                 message
//             ))
//         })
//         .chain(fern::log_file("program.log")?);

//     base_config.chain(file_config);

//     Ok(())
// }
