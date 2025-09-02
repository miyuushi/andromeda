use std::{fs, io::Write, str::FromStr};

use log::info;

pub mod api;
pub mod config;
pub mod errors;
pub mod exports;
pub mod utils;

pub mod logging {
  use std::fmt;

  use chrono::Local;
  use fern::FormatCallback;

  pub fn andromeda_stdout_logging_format(out: FormatCallback, message: &fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
      "[Andromeda] [{}] [{}] {}",
      record.level(),
      record.target(),
      message
    ))
  }

  pub fn andromeda_file_logging_format(out: FormatCallback, message: &fmt::Arguments, record: &log::Record) {
    out.finish(format_args!(
      "[Andromeda] {} [{}] [{}] {}",
      Local::now().format("%Y-%m-%d %H:%M:%S %:z"),
      record.level(),
      record.target(),
      message
    ))
  }
}
