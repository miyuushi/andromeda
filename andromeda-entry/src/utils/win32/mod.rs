use std::ffi::{CString, c_void};

use andromeda_common::utils::win32;
use windows::{
  Win32::{Foundation::HMODULE, System::LibraryLoader::GetProcAddress},
  core::PCSTR
};

pub(crate) mod dll;
pub(crate) mod iat;
pub(crate) mod module;
pub(crate) mod process;

/// Returns a module symbol's absolute address.
pub(crate) fn get_module_symbol_address(handle: usize, symbol: &str) -> Option<usize> {
  let symbol = CString::new(symbol).expect("CString had internal null byte present");
  unsafe {
    GetProcAddress(HMODULE(handle as *mut c_void), PCSTR(symbol.as_ptr() as *const u8)).map(|func| func as usize)
  }
}
