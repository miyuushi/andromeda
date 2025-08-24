mod exports;
mod util;

use minhook::MinHook;
use std::{error::Error, ffi::c_void, ptr, thread, time::Duration};
use windows::{
  Win32::{
    Foundation::HINSTANCE,
    System::{
      SystemServices::DLL_PROCESS_ATTACH,
      Threading::{CreateThread, THREAD_CREATION_FLAGS}
    }
  },
  core::BOOL
};

use crate::{
  exports::{DXGICreateFactoryFn, ORIG_DXGI_CREATE_FACTORY, hooked_dxgi_create_factory},
  util::{get_module_symbol_address, log}
};

pub fn add(left: u64, right: u64) -> u64 {
  left + right
}

unsafe fn install_hooks() {
  log("[Andromeda] Attempt to hook functions with MinHook");
  let hook_dxgi_create_factory = |address: usize| unsafe {
    let target: DXGICreateFactoryFn = std::mem::transmute(address);
    let trampoline = MinHook::create_hook(target as _, hooked_dxgi_create_factory as _)
      .expect("Failed to hook DXGICreateFactory!");

    ORIG_DXGI_CREATE_FACTORY.get_or_init(|| std::mem::transmute(trampoline));
    MinHook::enable_hook(target as _).unwrap();
  };

  let dxgi_create_factory_address = get_module_symbol_address("dxgi.dll", "DXGICreateFactory");
  match dxgi_create_factory_address {
    None => {
      log("[Andromeda] dxgi.dll not loaded in this process (GetModuleHandleA returned NULL).")
    }
    Some(address) => hook_dxgi_create_factory(address)
  }

  for _ in 0..10 {
    let dxgi_create_factory_address = get_module_symbol_address("dxgi.dll", "DXGICreateFactory");
    if let Some(address) = dxgi_create_factory_address {
      log("[Andromeda] dxgi.dll found on retry; attempting hook again if needed");
      hook_dxgi_create_factory(address);
      break;
    }
    thread::sleep(Duration::from_millis(200));
  }

  log("[Andromeda] DXGICreateFactory hooked");
}

unsafe fn initialize() -> Result<(), Box<dyn Error>> {
  unsafe {
    install_hooks();
  }

  Ok(())
}

unsafe extern "system" fn thread_main(_: *mut c_void) -> u32 {
  log("[Andromeda] thread_main running");
  // A console may be useful for printing to 'stdout'
  unsafe {
    initialize();
  }

  log("[Andromeda] thread_main finished");
  0
}

#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables)]
pub unsafe extern "system" fn DllMain(_module: HINSTANCE, call_reason: u32, _: *mut ()) -> BOOL {
  if call_reason == DLL_PROCESS_ATTACH {
    log("[Andromeda] DllMain: PROCESS_ATTACH");

    // Preferably a thread should be created here instead, since as few
    // operations as possible should be performed within `DllMain`.

    unsafe {
      let handle = CreateThread(
        None,
        0,
        Some(thread_main),
        None,
        THREAD_CREATION_FLAGS(0),
        None
      );

      if handle.is_err() {
        // fallback: try std::thread if CreateThread failed (unlikely)
        log("[Andromeda] CreateThread failed; falling back to std::thread");
        std::thread::spawn(|| unsafe {
          match crate::thread_main(ptr::null_mut()) {
            0 => 0,
            err => {
              println!("[Andromeda] Error occurred when injecting: {}", err);
              1
            }
          }
        });
      } else {
        log("[Andromeda] background thread spawned");
      }
    }
  }
  true.into()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_works() {
    let result = add(2, 2);
    assert_eq!(result, 4);
  }
}
