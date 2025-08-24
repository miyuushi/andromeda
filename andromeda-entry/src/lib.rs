use windows::Win32::System::SystemServices::{
  DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH
};
use windows::core::{GUID, HRESULT, IUnknown, PCSTR, PCWSTR};
use windows::{
  Win32::Foundation::*, Win32::System::LibraryLoader::*,
  Win32::System::SystemInformation::GetSystemDirectoryW,
  Win32::UI::WindowsAndMessaging::MessageBoxA
};
// use retour::static_detour;
use minhook::{MH_STATUS, MinHook};
use once_cell::sync::OnceCell;
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use std::{ffi::CString, ffi::c_int, iter, mem};

type DirectInput8CreateFn = unsafe extern "system" fn(
  hinst: HINSTANCE,
  dw_version: u32,
  riidltf: *const GUID,
  ppv_out: *mut *mut c_void,
  punk_outer: *mut IUnknown
) -> HRESULT;

static ORIGINAL_DIRECT_INPUT8_CREATE: OnceCell<DirectInput8CreateFn> = OnceCell::new();
static REAL_DLL: OnceLock<usize> = OnceLock::new();
static LOADED_PAYLOAD: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone)]
struct ProxyError(String);

impl fmt::Display for ProxyError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for ProxyError {}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DirectInput8Create(
  hinst: HINSTANCE,
  dw_version: u32,
  riidltf: *const GUID,
  ppv_out: *mut *mut c_void,
  punk_outer: *mut IUnknown
) -> HRESULT {
  unsafe {
    let _ = windows::Win32::System::Console::AllocConsole();
  }
  println!("[Proxy] DirectInput8Create called (dw_version={dw_version})");
  unsafe {
    let _ = load_real_dll();
  }

  LOADED_PAYLOAD.get_or_init(|| {
    let custom_path =
      "/home/mimi/Documents/Projects/andromeda/target/x86_64-pc-windows-gnu/debug/andromeda.dll";
    let wide: Vec<u16> = custom_path.encode_utf16().chain([0]).collect();
    unsafe {
      let _ = LoadLibraryW(PCWSTR::from_raw(wide.as_ptr()));
    }
  });

  let result = unsafe {
    ORIGINAL_DIRECT_INPUT8_CREATE.get().unwrap()(hinst, dw_version, riidltf, ppv_out, punk_outer)
  };

  if result.is_ok() && !ppv_out.is_null() {
    println!("[Proxy] Got IDirectInput8 interface at {ppv_out:?}");
    // At this point you could wrap or hook the returned COM object
  }

  result
}

/// Called when the DLL is attached to the process.
unsafe fn load_real_dll() -> Result<(), Box<dyn Error>> {
  if REAL_DLL.get().is_some() {
    return Err(Box::new(ProxyError(
      "Can't load real DLL twice!".to_owned()
    )));
  }

  let system_path = get_system32_path();
  let module_name = Path::new(&system_path).join("dinput8.dll");
  let address = get_module_symbol_address(module_name.to_str().unwrap(), "DirectInput8Create")
    .expect("Could not find 'DirectInput8Create' address");

  REAL_DLL.set(address).unwrap();

  unsafe { ORIGINAL_DIRECT_INPUT8_CREATE.get_or_init(|| std::mem::transmute(address)) };

  Ok(())
}

pub fn get_system32_path() -> String {
  // Allocate a buffer for the path (MAX_PATH characters)
  let mut buffer = [0u16; MAX_PATH as usize];

  unsafe {
    let len = GetSystemDirectoryW(Some(&mut buffer));
    if len == 0 {
      panic!("Failed to get system directory");
    }

    // Convert UTF-16 buffer to Rust String
    let path = String::from_utf16_lossy(&buffer[..len as usize]);
    println!("System directory: {}", path);

    path
  }
}

/// Returns a module symbol's absolute address.
fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
  let module = module
    .encode_utf16()
    .chain(iter::once(0))
    .collect::<Vec<u16>>();
  let symbol = CString::new(symbol).unwrap();
  unsafe {
    let handle = LoadLibraryW(PCWSTR(module.as_ptr() as _)).unwrap();
    GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)).map(|func| func as usize)
  }
}

// DirectInput8Create=FORWARDER_DirectInput8Create		@1
// D3D11CreateDevice=FORWARDER_D3D11CreateDevice		@5
// CreateDXGIFactory=FORWARDER_CreateDXGIFactory		@10
// CreateDXGIFactory1=FORWARDER_CreateDXGIFactory1		@11
// CreateDXGIFactory2=FORWARDER_CreateDXGIFactory2		@12
