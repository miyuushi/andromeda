use std::ffi::CString;

use windows::{
  Win32::{Foundation::MAX_PATH, System::SystemInformation::GetSystemDirectoryW},
  core::{PCSTR, PCWSTR}
};

pub fn get_system32_path() -> String {
  let mut buffer = [0u16; MAX_PATH as usize];

  unsafe {
    let len = GetSystemDirectoryW(Some(&mut buffer));
    if len == 0 {
      panic!("Failed to get system directory");
    }

    String::from_utf16_lossy(&buffer[..len as usize])
  }
}

pub fn widestring<S: Into<String>>(value: S) -> Vec<u16> {
  value
    .into()
    .encode_utf16()
    .chain(std::iter::once(0))
    .collect::<Vec<u16>>()
}
