pub(crate) mod dx11;

use andromeda_common::{
  exports::{D3D11CreateDeviceAndSwapChainFn, D3D11CreateDeviceFn},
  utils::win32
};
use log::info;
use once_cell::sync::OnceCell;
use std::{
  error::Error,
  ffi::{CString, c_void}
};
use windows::Win32::{
  Foundation::{HMODULE, NTSTATUS},
  System::LibraryLoader::GetProcAddress
};
use windows_core::PCSTR;

use crate::{
  hooks::dx11::{
    DX11_HOOKS, d3d11_create_device_and_swapchain_hook, d3d11_create_device_hook, dxgi_create_factory_hook,
    dxgi_create_factory1_hook, dxgi_create_factory2_hook
  },
  util::get_module_symbol_address
};

pub(crate) type LoadLibraryWFn = unsafe extern "system" fn(lpLibFileName: *const u16) -> HMODULE;

pub(crate) static ORIG_LOADLIBRARYW: OnceCell<LoadLibraryWFn> = OnceCell::new();

pub(crate) unsafe extern "system" fn hook_loadlibraryw(lpLibFileName: *const u16) -> HMODULE {
  let name = widestring::U16CStr::from_ptr_str(lpLibFileName).to_string_lossy();
  if name.to_lowercase().contains("d3d11.dll") || name.to_lowercase().contains("dxgi.dll") {
    println!("[+] {} loaded, installing DX hooks", name);
    // install_dxgi_hooks();
  }
  ORIG_LOADLIBRARYW.get().unwrap()(lpLibFileName)
}

/// Generic vtable hook installer
pub(crate) unsafe fn hook_vtable_method(
  vtable: *mut *mut c_void,
  index: usize,
  detour: *mut c_void
) -> Result<*mut c_void, String> {
  let target = *vtable.add(index) as *mut c_void;

  let trampoline =
    min_hook_rs::create_hook(target, detour).map_err(|_| format!("Failed to hook vtable index {}", index))?;
  min_hook_rs::enable_hook(target).map_err(|_| "Failed to enable hook".to_string())?;

  Ok(trampoline as *mut c_void)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct UNICODE_STRING {
  pub Length: u16,
  pub MaximumLength: u16,
  pub Buffer: *const u16
}

#[repr(C)]
pub union LDR_DLL_NOTIFICATION_DATA {
  pub Loaded: LDR_DLL_LOADED_NOTIFICATION_DATA,
  pub Unloaded: LDR_DLL_UNLOADED_NOTIFICATION_DATA
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LDR_DLL_LOADED_NOTIFICATION_DATA {
  pub Flags: u64,
  pub FullDllName: UNICODE_STRING,
  pub BaseDllName: UNICODE_STRING,
  pub DllBase: *mut c_void,
  pub SizeOfImage: u64
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LDR_DLL_UNLOADED_NOTIFICATION_DATA {
  pub Flags: u64,
  pub FullDllName: UNICODE_STRING,
  pub BaseDllName: UNICODE_STRING,
  pub DllBase: *mut c_void,
  pub SizeOfImage: u64
}

type LdrDllNotificationFunction =
  unsafe extern "system" fn(reason: u64, data: *const LDR_DLL_NOTIFICATION_DATA, context: *mut c_void);

#[link(name = "ntdll")]
unsafe extern "system" {
  pub(crate) unsafe fn LdrRegisterDllNotification(
    Flags: u64,
    NotificationFunction: LdrDllNotificationFunction,
    Context: *mut c_void,
    Cookie: *mut *mut c_void
  ) -> NTSTATUS;

  pub(crate) unsafe fn LdrUnregisterDllNotification(Cookie: *mut c_void) -> NTSTATUS;
}

unsafe fn unicode_to_string(us: &UNICODE_STRING) -> String {
  if us.Buffer.is_null() || us.Length == 0 {
    return String::new();
  }
  let len = (us.Length / 2) as usize; // length in WCHARs
  let slice = std::slice::from_raw_parts(us.Buffer, len);
  String::from_utf16_lossy(slice)
}

pub(crate) unsafe extern "system" fn dll_notify(
  reason: u64,
  data: *const LDR_DLL_NOTIFICATION_DATA,
  _context: *mut c_void
) {
  if reason == 1 {
    // DLL loaded
    let base_name = unicode_to_string(&(*data).Loaded.BaseDllName);
    let full_name = unicode_to_string(&(*data).Loaded.FullDllName);

    unsafe {
      info!(
        "base: {:?}\n",
        std::slice::from_raw_parts(
          (*data).Loaded.BaseDllName.Buffer,
          (*data).Loaded.BaseDllName.Length as usize
        )
      )
    };

    if base_name == "d3d11.dll" || base_name == "dxgi.dll" {
      println!("[+] Detected {} loaded at {:p}", base_name, (*data).Loaded.DllBase);

      // Resolve functions here
      // Example:
      let d3d11_create_device = unsafe {
        GetProcAddress(
          windows::Win32::Foundation::HMODULE((*data).Loaded.DllBase),
          PCSTR(
            CString::new("D3D11CreateDevice")
              .expect("CString had internal null byte present")
              .as_ptr() as *const u8
          )
        )
      };

      // let dxgi_create_factory = GetProcAddress((*data).DllBase, b"CreateDXGIFactory\0");
    }
  }
}

pub(crate) unsafe fn try_install_dx11_hooks() -> Result<(), Box<dyn Error>> {
  if DX11_HOOKS.dxgi_create_factory.get().is_none() {
    let dxgi_create_factory_address = get_module_symbol_address("dxgi.dll", "CreateDXGIFactory");
    if let Some(address) = dxgi_create_factory_address {
      info!("CreateDXGIFactory");
      let (trampoline, target) =
        min_hook_rs::create_hook_api_ex("dxgi.dll", "CreateDXGIFactory", dxgi_create_factory_hook as *mut c_void)
          .map_err(|e| "Failed to hook CreateDXGIFactory!")?;

      DX11_HOOKS
        .dxgi_create_factory
        .get_or_init(|| std::mem::transmute(trampoline));
      min_hook_rs::enable_hook(target)?;
    }
  }

  if DX11_HOOKS.dxgi_create_factory1.get().is_none() {
    let dxgi_create_factory1_address = get_module_symbol_address("dxgi.dll", "CreateDXGIFactory1");
    if let Some(address) = dxgi_create_factory1_address {
      info!("CreateDXGIFactory1");
      let (trampoline, target) = min_hook_rs::create_hook_api_ex(
        "dxgi.dll",
        "CreateDXGIFactory1",
        dxgi_create_factory1_hook as *mut c_void
      )
      .map_err(|e| "Failed to hook CreateDXGIFactory1!")?;

      DX11_HOOKS
        .dxgi_create_factory1
        .get_or_init(|| std::mem::transmute(trampoline));
      min_hook_rs::enable_hook(target)?;
    }
  }

  if DX11_HOOKS.dxgi_create_factory2.get().is_none() {
    let dxgi_create_factory2_address = get_module_symbol_address("dxgi.dll", "CreateDXGIFactory2");
    if let Some(address) = dxgi_create_factory2_address {
      info!("CreateDXGIFactory2");
      let (trampoline, target) = min_hook_rs::create_hook_api_ex(
        "dxgi.dll",
        "CreateDXGIFactory2",
        dxgi_create_factory2_hook as *mut c_void
      )
      .map_err(|e| "Failed to hook CreateDXGIFactory2!")?;

      DX11_HOOKS
        .dxgi_create_factory2
        .get_or_init(|| std::mem::transmute(trampoline));
      min_hook_rs::enable_hook(target)?;
    }
  }

  if DX11_HOOKS.d3d11_create_device.get().is_none() {
    let d3d11_create_device_address = get_module_symbol_address("d3d11.dll", "D3D11CreateDevice");
    if let Some(address) = d3d11_create_device_address {
      info!("Hooked D3D11CreateDevice");
      unsafe {
        let target: D3D11CreateDeviceFn = std::mem::transmute(address);
        let (trampoline, target) =
          min_hook_rs::create_hook_api_ex("d3d11.dll", "D3D11CreateDevice", d3d11_create_device_hook as *mut _)
            .map_err(|e| "Failed to hook D3D11CreateDevice!")?;

        DX11_HOOKS
          .d3d11_create_device
          .get_or_init(|| std::mem::transmute(trampoline));
        min_hook_rs::enable_hook(target)?;
      }
    }
  }

  if DX11_HOOKS.d3d11_create_device_and_sc.get().is_none() {
    let d3d11_create_device_and_sc_address = get_module_symbol_address("d3d11.dll", "D3D11CreateDeviceAndSwapChain");
    if let Some(address) = d3d11_create_device_and_sc_address {
      info!("Hooked D3D11CreateDeviceAndSwapChain");
      unsafe {
        let target: D3D11CreateDeviceAndSwapChainFn = std::mem::transmute(address);
        let (trampoline, target) = min_hook_rs::create_hook_api(
          "d3d11.dll",
          "D3D11CreateDeviceAndSwapChain",
          d3d11_create_device_and_swapchain_hook as *mut c_void
        )
        .map_err(|e| "Failed to hook D3D11CreateDeviceAndSwapChain!")?;

        DX11_HOOKS
          .d3d11_create_device_and_sc
          .get_or_init(|| std::mem::transmute(trampoline));
        min_hook_rs::enable_hook(target)?;
      }
    }
  }

  Ok(())
}
