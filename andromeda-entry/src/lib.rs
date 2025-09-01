mod entrypoint;
mod patches;
mod util;
mod utils;

use andromeda_common::errors::AndromedaError;
use andromeda_common::logging::{andromeda_file_logging_format, andromeda_stdout_logging_format};
use andromeda_common::utils::win32;
use andromeda_common::{
  AndromedaConfig, create_andromeda_config, get_andromeda_config, get_andromeda_config_path, get_andromeda_loader_path,
  get_andromeda_log_path
};
use log::{error, info};
use once_cell::sync::OnceCell;
use std::error::Error;
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::{ffi::CString, ffi::c_int, iter, mem};
use std::{fmt, io};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL};
use windows::Win32::Graphics::Direct3D11::{D3D11_CREATE_DEVICE_FLAG, ID3D11Device, ID3D11DeviceContext};
use windows::Win32::Graphics::Dxgi::IDXGIAdapter;
use windows::Win32::System::SystemServices::{
  DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH
};
use windows::Win32::System::Threading::{CreateThread, THREAD_CREATION_FLAGS};
use windows::core::{GUID, HRESULT, IUnknown, PCSTR, PCWSTR};
use windows::{Win32::Foundation::*, Win32::System::LibraryLoader::*};

use crate::patches::apply_all_patches;
use crate::utils::win32::module::LoadedModule;

type DirectInput8CreateFn = unsafe extern "system" fn(
  hinst: HINSTANCE,
  dw_version: u32,
  riidltf: *const GUID,
  ppv_out: *mut *mut c_void,
  punk_outer: *mut IUnknown
) -> HRESULT;

static PAYLOAD_LOADED: AtomicBool = AtomicBool::new(false);
static H_MODULE: OnceLock<Mutex<LoadedModule>> = OnceLock::new();

type InjectAndromedaEntrypointFn = unsafe extern "system" fn() -> bool;

fn ensure_payload_loaded(config: AndromedaConfig) -> Result<(), AndromedaError> {
  if PAYLOAD_LOADED.load(Ordering::Acquire) {
    return Ok(());
  }
  const PAYLOAD_NAME: &str = "andromeda.dll";
  let loader_path = get_andromeda_loader_path(config).map_or(PAYLOAD_NAME.into(), |path| path.join(PAYLOAD_NAME));
  let loader_path = loader_path
    .to_str()
    .ok_or_else(|| AndromedaError::Path("Invalid payload path was specified".to_string()))?;
  info!("Loader path: {}", loader_path);
  let wide = win32::widestring(loader_path);

  unsafe {
    let payload = LoadLibraryW(PCWSTR::from_raw(wide.as_ptr()));
    if let Ok(payload) = payload {
      let proc = GetProcAddress(
        payload,
        PCSTR(
          CString::new("inject_andromeda_entrypoint")
            .expect("CString had internal null byte present")
            .as_ptr() as *const u8
        )
      );
      if let Some(proc) = proc {
        let inject_andromeda_entrypoint: InjectAndromedaEntrypointFn = std::mem::transmute(proc);
        inject_andromeda_entrypoint();
      }
    } else if let Err(err) = payload {
      error!("Error loading payload!: {}", err);
    }
    // Mark payload as loaded even on failure to prevent retry storming
    PAYLOAD_LOADED.store(true, Ordering::Release);
  }
  Ok(())
}

fn init_console() -> io::Result<()> {
  unsafe {
    // Allocate a new console
    windows::Win32::System::Console::AllocConsole()?;

    let mode = CString::new("w").unwrap();
    let conout = CString::new("CONOUT$").unwrap();

    // Attach stdout/stderr to the console
    libc::freopen(
      conout.as_ptr(),
      mode.as_ptr(),
      libc::fdopen(libc::STDOUT_FILENO, mode.as_ptr())
    );
    libc::freopen(
      conout.as_ptr(),
      mode.as_ptr(),
      libc::fdopen(libc::STDERR_FILENO, mode.as_ptr())
    );
  }

  Ok(())
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

unsafe extern "system" fn thread_main(_: *mut c_void) -> u32 {
  unsafe {
    init_console();
  }
  min_hook_rs::initialize();

  // let log_dir = get_andromeda_log_path().unwrap_or_else(|| "logs".into());

  // let log_file = OpenOptions::new()
  //   .write(true)
  //   .create(true)
  //   .truncate(true)
  //   .open(log_dir.join("Andromeda.Entry.log"));

  // // current log will always be Andromeda.Entry.log
  // let file_appender =rolling::daily(&log_dir, "Andromeda.Entry.log");
  // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

  // let stdout_layer = layer()
  //   .with_writer(std::io::stdout)
  //   .with_timer(ChronoLocal::rfc_3339())
  //   .with_target(false) // hide target if you want
  //   .with_level(true);
  // let file_layer = layer()
  //   .with_writer(non_blocking)
  //   .with_timer(ChronoLocal::new(
  //     // similar to your "%Y-%m-%d_%H-%M-%S"
  //     (|| Local::now().format("%Y-%m-%d_%H-%M-%S").to_string())()
  //   ))
  //   .with_target(true)
  //   .with_level(true);

  // let logger = tracing_subscriber::registry()
  //   .with(
  //     EnvFilter::builder()
  //       .with_default_directive(LevelFilter::INFO.into())
  //       .from_env_lossy()
  //   )
  //   .with(stdout_layer)
  //   .with(file_layer)
  //   .init();

  // let logger = flexi_logger::Logger::try_with_env_or_str("info").and_then(|l| {
  //   l.log_to_file(
  //     FileSpec::default()
  //       .directory(
  //         get_andromeda_config_path()
  //           .map(|c| c.join("logs"))
  //           .unwrap_or("logs".into())
  //       )
  //       .basename("Andromeda.Entry")
  //   )
  //   .log_to_stdout()
  //   .write_mode(WriteMode::Direct)
  //   .format(andromeda_logging_format)
  //   .rotate(
  //     Criterion::Age(Age::Day),
  //     Naming::TimestampsCustomFormat { current_infix: None, format: "%Y-%m-%d_%H-%M-%S" },
  //     Cleanup::KeepLogFiles(20)
  //   )
  //   .start()
  // });

  match init_logger() {
    Ok(_) => info!("Logger initialized!"),
    Err(ref e) => error!("Failed to initialize logger! {e}")
  }

  let config = match get_andromeda_config() {
    Some(config) => config,
    None => create_andromeda_config().unwrap_or_default()
  };

  info!("Successfully created or read config: {:?}", config);

  // match manual_map_dll("andromeda.dll") {
  //   Ok(addr) => {
  //     error!("Manual-mapped at {:p}", addr);
  //   }
  //   Err(e) => {
  //     eerror!("Manual map failed: {}", e);
  //   }
  // }
  apply_all_patches();

  let _ = ensure_payload_loaded(config);

  0
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(
  _hinst: windows::Win32::Foundation::HINSTANCE,
  reason: u32,
  _reserved: *mut std::ffi::c_void
) -> i32 {
  if reason == DLL_PROCESS_ATTACH {
    unsafe {
      DisableThreadLibraryCalls(_hinst.into());
      let handle = CreateThread(None, 0, Some(thread_main), None, THREAD_CREATION_FLAGS(0), None);

      if handle.is_err() {
        // fallback: try std::thread if CreateThread failed (unlikely)
        error!("CreateThread failed; falling back to std::thread");
        std::thread::spawn(|| match crate::thread_main(std::ptr::null_mut()) {
          0 => 0,
          err => {
            error!("Error occurred when injecting: {}", err);
            1
          }
        });
      } else {
        info!("Background thread spawned");
      }
    }

    H_MODULE.get_or_init(|| Mutex::new(LoadedModule::default()));
    let module = H_MODULE.get();
    if let Some(m) = module {
      if let Ok(mut h) = m.lock() {
        h.handle.attach(_hinst.into(), false);
      }
    }
    // let result = unsafe { patch_entry_point_for_injection(GetCurrentProcess()) };

    // unsafe { (*result).LoadInstalledXivAlexDllOnly = true };
  }
  1
}

// DirectInput8Create=FORWARDER_DirectInput8Create		@1
// D3D11CreateDevice=FORWARDER_D3D11CreateDevice		@5
// CreateDXGIFactory=FORWARDER_CreateDXGIFactory		@10
// CreateDXGIFactory1=FORWARDER_CreateDXGIFactory1		@11
// CreateDXGIFactory2=FORWARDER_CreateDXGIFactory2		@12
