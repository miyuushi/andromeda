pub mod win32;

// Basic logging for the entrypoint where we can't use `flexi_logger` for stdout/stderr
macro_rules! log {
  ($($args: tt)*) => {
    println!("[Andromeda] {}", chrono::Local::now().format(TS_DASHES_BLANK_COLONS_DOT_BLANK), format_args!($($args)*));
    info!($($args)*);
  }
}
