use std::{fs, io::Write};

use log::info;
use serde::{Deserialize, Serialize};

use crate::errors::AndromedaError;

pub mod errors;
pub mod exports;
pub mod utils;

pub mod logging {
  use std::{fmt, fs::OpenOptions};

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
      // humantime::format_rfc3339(std::time::SystemTime::now()),
      Local::now().format("%Y-%m-%d %H:%M:%S %:z"),
      record.level(),
      record.target(),
      message
    ))
  }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct AndromedaPlugin {
  enabled: bool,
  name: String,
  id: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AndromedaConfig {
  #[serde(rename = "devBuild")]
  dev_build: bool,
  #[serde(rename = "latestVersion")]
  latest_version: String,
  #[serde(rename = "checkForUpdates")]
  check_for_updates: bool,
  #[serde(rename = "plugins")]
  plugins: Vec<AndromedaPlugin>,
  #[serde(rename = "seenPlugins")]
  seen_plugins: Vec<String>
}

impl Default for AndromedaConfig {
  fn default() -> Self {
    Self {
      dev_build: true,
      latest_version: "0.0.1".to_string(),
      check_for_updates: true,
      plugins: Default::default(),
      seen_plugins: Default::default()
    }
  }
}

pub fn get_andromeda_loader_path(config: AndromedaConfig) -> Option<std::path::PathBuf> {
  get_andromeda_config_path().map(|path| {
    path
      .join("loader")
      .join(if config.dev_build { "dev".into() } else { config.latest_version })
  })
}

pub fn get_andromeda_log_path() -> Option<std::path::PathBuf> {
  let path = get_andromeda_config_path().map(|c| c.join("logs"))?;
  fs::create_dir_all(&path).ok()?;
  Some(path)
}

pub fn get_andromeda_config_path() -> Option<std::path::PathBuf> {
  let path = dirs::config_dir().map(|dir| dir.join("Andromeda").to_path_buf());
  if let Some(ref andromeda_path) = path {
    fs::create_dir_all(andromeda_path).ok()?
  }
  path
}

pub fn get_andromeda_config() -> Option<AndromedaConfig> {
  if let Some(andromeda_path) = get_andromeda_config_path() &&
    let Ok(true) = fs::exists(andromeda_path.join("andromeda_config.json"))
  {
    let file = fs::File::open(andromeda_path.join("andromeda_config.json")).ok()?;
    serde_json::from_reader(file).ok()?
  }
  None
}

pub fn create_andromeda_config() -> Result<AndromedaConfig, AndromedaError> {
  if let Some(andromeda_path) = get_andromeda_config_path() {
    let file_path = andromeda_path.join("andromeda_config.json");
    fs::create_dir_all(&andromeda_path)?;
    if let Ok(false) = fs::exists(&file_path) {
      info!("File path: {}", file_path.to_str().unwrap());
      let mut file = fs::File::create(file_path)?;
      let default_config = serde_json::to_string_pretty(&AndromedaConfig::default())?;
      file.write_all(default_config.as_bytes())?;
    }
  }

  Ok(AndromedaConfig::default())
}
