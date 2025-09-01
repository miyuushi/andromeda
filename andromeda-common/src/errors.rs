use std::{fmt, io};

pub enum AndromedaError {
  Hooking(String),
  MinHook(String),
  IO(String),
  JSON(String),
  Logger(String),
  Path(String)
}

impl fmt::Display for AndromedaError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      AndromedaError::Hooking(msg) => write!(f, "An error occurred while attempting to hook a function: {}", msg),
      AndromedaError::MinHook(msg) => write!(f, "A MinHook error occurred: {}", msg),
      AndromedaError::IO(msg) => write!(f, "An error occurred: {}", msg),
      AndromedaError::JSON(msg) => write!(f, "A JSON error occurred: {}", msg),
      AndromedaError::Logger(msg) => write!(f, "A logger error occurred: {}", msg),
      AndromedaError::Path(msg) => write!(f, "A path error has occurred: {}", msg)
    }
  }
}

impl From<io::Error> for AndromedaError {
  fn from(error: io::Error) -> Self {
    AndromedaError::IO(error.to_string())
  }
}

impl From<min_hook_rs::HookError> for AndromedaError {
  fn from(error: min_hook_rs::HookError) -> Self {
    AndromedaError::MinHook(error.to_string())
  }
}

impl From<serde_json::Error> for AndromedaError {
  fn from(error: serde_json::Error) -> Self {
    AndromedaError::JSON(error.to_string())
  }
}

impl From<log::SetLoggerError> for AndromedaError {
  fn from(error: log::SetLoggerError) -> Self {
    AndromedaError::Logger(error.to_string())
  }
}
