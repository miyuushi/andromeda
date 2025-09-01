mod egui_backend;
mod exports;
mod hooks;
mod internal;
mod util;

use andromeda_common::{
  errors::AndromedaError,
  exports::{D3D11CreateDeviceAndSwapChainFn, D3D11CreateDeviceFn},
  get_andromeda_config_path, get_andromeda_log_path,
  logging::{andromeda_file_logging_format, andromeda_stdout_logging_format}
};
use chrono::Local;
use log::info;
use std::{error::Error, ffi::c_void, fmt, fs::OpenOptions, io, ptr, sync::Mutex, thread, time::Duration};
use windows::{
  Win32::{
    Foundation::HINSTANCE,
    System::{
      LibraryLoader::{GetModuleHandleA, GetProcAddress},
      SystemServices::DLL_PROCESS_ATTACH,
      Threading::{CreateThread, THREAD_CREATION_FLAGS}
    }
  },
  core::BOOL
};

use crate::{
  hooks::try_install_dx11_hooks,
  internal::{INTERFACES, interfaces::Interfaces},
  util::log
};

unsafe fn try_install_hooks() -> Result<(), AndromedaError> {
  info!("Attempt to hook functions with MinHook");
  min_hook_rs::initialize()?;

  for _ in 0..40 {
    if unsafe { try_install_dx11_hooks().is_ok() } {
      info!("Hooked DX11 functions successfully!");
      return Ok(());
    }

    thread::sleep(Duration::from_millis(50));
  }

  Err(AndromedaError::Hooking("Failed to install any hooks!".to_string()))
}

pub fn init_logger() -> Result<(), AndromedaError> {
  let log_dir = get_andromeda_log_path().unwrap_or_else(|| "logs".into());

  let log_file = OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .open(log_dir.join("Andromeda.Entry.log"))?;

  let logger = fern::Dispatch::new()
    .level(log::LevelFilter::Debug);

  // Output to stdout and files
  let file_config = fern::Dispatch::new()
    .format(andromeda_file_logging_format)
    .chain(log_file);
  let stdout_config = fern::Dispatch::new()
    .format(andromeda_stdout_logging_format)
    .chain(std::io::stdout());

  logger.chain(file_config).chain(stdout_config).apply()?;

  Ok(())
}

#[unsafe(no_mangle)]
unsafe extern "system" fn inject_andromeda_entrypoint() -> bool {
  // Initialize singletons
  INTERFACES.get_or_init(|| Mutex::new(Interfaces::new()));

  match init_logger() {
    Ok(_) => info!("Logger initialized!"),
    Err(e) => println!("Failed to initialize logger! {e}")
  }

  // Setup Andromeda and install hooks
  unsafe {
    try_install_hooks();
  }

  true
}

unsafe extern "system" fn thread_main(_: *mut c_void) -> u32 {
  log("[Andromeda] thread_main running");
  // A console may be useful for printing to 'stdout'
  unsafe {
    inject_andromeda_entrypoint();
  }

  log("[Andromeda] thread_main finished");
  0
}

#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables)]
pub unsafe extern "system" fn DllMain(_module: HINSTANCE, call_reason: u32, _: *mut ()) -> BOOL {
  if call_reason == DLL_PROCESS_ATTACH {
    log("[Andromeda] DllMain: PROCESS_ATTACH");

    // unsafe {
    //   let mut cookie: *mut c_void = std::ptr::null_mut();
    //   let status = LdrRegisterDllNotification(0, dll_notify, std::ptr::null_mut(), &mut cookie);
    //   if status.0 != 0 {
    //       println!("[-] Failed to register DLL notification: 0x{:X}", status.0);
    //   } else {
    //       println!("[+] DLL notification registered");
    //   }

    //   // Call LdrUnregisterDllNotification(cookie) when your DLL unloads
    // }

    // unsafe {
    //   let handle = CreateThread(
    //     None,
    //     0,
    //     Some(thread_main),
    //     None,
    //     THREAD_CREATION_FLAGS(0),
    //     None
    //   );

    //   if handle.is_err() {
    //     // fallback: try std::thread if CreateThread failed (unlikely)
    //     log("[Andromeda] CreateThread failed; falling back to std::thread");
    //     std::thread::spawn(|| {
    //       match crate::thread_main(ptr::null_mut()) {
    //         0 => 0,
    //         err => {
    //           println!("[Andromeda] Error occurred when injecting: {}", err);
    //           1
    //         }
    //       }
    //     });
    //   } else {
    //     log("[Andromeda] background thread spawned");
    //   }
    // }
  }
  true.into()
}
