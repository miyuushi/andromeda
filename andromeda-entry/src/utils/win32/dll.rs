use std::{path::PathBuf, sync::OnceLock};

use andromeda_common::utils::win32::{self, get_system32_path};
use windows::{
  Win32::System::LibraryLoader::LoadLibraryW,
  core::{HSTRING, PCWSTR}
};

use crate::utils::win32::get_module_symbol_address;

pub struct RealDll {
  name: &'static str,
  handle: OnceLock<usize>
}

impl RealDll {
  pub const fn new(name: &'static str) -> Self {
    Self {
      name,
      handle: OnceLock::new()
    }
  }

  fn load(&self) -> usize {
    *self.handle.get_or_init(|| {
      let system_path = get_system32_path();
      let module_name: PathBuf = [system_path, self.name.to_string()].iter().collect();
      let module = win32::widestring(module_name.to_str().unwrap());
      unsafe { LoadLibraryW(PCWSTR(HSTRING::from_wide(&module).as_ptr())).unwrap().0 as usize }
    })
  }

  pub fn get_orig_fn<T>(&self, symbol: &'static str) -> T {
    static CACHE: OnceLock<usize> = OnceLock::new();
    let addr = *CACHE.get_or_init(|| {
      get_module_symbol_address(self.load(), symbol).expect(format!("Could not find '{}' address", symbol).as_str())
    });
    unsafe { std::mem::transmute_copy(&addr) }
  }
}
