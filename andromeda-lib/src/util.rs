use andromeda_common::utils::win32;
use std::{
  ffi::{CString, c_void},
  fs::{File, OpenOptions},
  io::Write,
  iter,
  sync::Mutex
};
use windows::{
  Win32::{
    Foundation::{HLOCAL, LocalFree},
    System::{
      Diagnostics::Debug::{
        FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS, FormatMessageW
      },
      LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress}
    }
  },
  core::{HRESULT, PCSTR, PWSTR}
};
use windows_core::PCWSTR;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

// ---------- simple logger ----------
pub(crate) fn log<S: AsRef<str> + std::fmt::Display>(msg: S) {
  let _ = std::fs::create_dir_all("C:\\temp");
  let mut guard = LOG_FILE.lock().unwrap();
  if let Some(mut f) = guard.take() {
    let _ = writeln!(f, "{}", msg);
    *guard = Some(f);
  } else if let Ok(mut f) = OpenOptions::new()
    .create(true)
    .append(true)
    .open("C:\\temp\\payload_log.txt")
  {
    let _ = writeln!(f, "{}", msg);
    *guard = Some(f);
  }
  println!("{}", msg);
}

pub fn hresult_to_string(hr: HRESULT) -> String {
  unsafe {
    let mut buf: PWSTR = PWSTR::null();

    let len = FormatMessageW(
      FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_IGNORE_INSERTS,
      None,
      hr.0 as u32, // HRESULT is i32 under the hood
      0,
      PWSTR(buf.0),
      0,
      None
    );

    if len == 0 || buf.is_null() {
      return format!("HRESULT 0x{:08X}", hr.0);
    }

    // Take ownership of the buffer and turn it into a Rust String
    let slice = std::slice::from_raw_parts(buf.0, len as usize);
    let mut message = String::from_utf16_lossy(slice);

    // Clean up the allocated buffer
    LocalFree(Some(HLOCAL(buf.0 as *mut c_void)));

    // Strip trailing \r\n if present
    message = message.trim_end().to_string();

    format!("HRESULT 0x{:08X}: {}", hr.0, message)
  }
}

/// Returns a module symbol's absolute address.
pub(crate) fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
  let symbol = CString::new(symbol).expect("CString had internal null byte present");
  let module = win32::widestring(module);
  unsafe {
    let handle = GetModuleHandleW(PCWSTR(module.as_ptr())).ok();
    GetProcAddress(handle?, PCSTR(symbol.as_ptr() as *const u8)).map(|func| func as usize)
  }
}
