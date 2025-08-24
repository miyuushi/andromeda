use std::{
  ffi::CString,
  fs::{File, OpenOptions},
  io::Write,
  iter,
  sync::Mutex
};
use windows::{
  Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
  core::PCSTR
};

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

// ---------- simple file logger ----------
pub(crate) fn log(msg: &str) {
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

/// Returns a module symbol's absolute address.
pub(crate) fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
  let module = module
    .encode_utf16()
    .chain(iter::once(0))
    .collect::<Vec<u16>>();
  let symbol = CString::new(symbol).unwrap();
  unsafe {
    let handle = GetModuleHandleA(PCSTR(module.as_ptr() as _)).ok();
    GetProcAddress(handle?, PCSTR(symbol.as_ptr() as _)).map(|func| func as usize)
  }
}
