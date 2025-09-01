use std::ffi::CString;
use std::ffi::c_void;
use windows::Win32::Devices::HumanInterfaceDevice::DIRECTINPUT_VERSION;
use windows::Win32::Devices::HumanInterfaceDevice::IDirectInput8W;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::core::GUID;
use windows::core::HRESULT;
use windows::core::IUnknown;
use windows::core::Interface;
use windows::core::PCSTR;

type DirectInput8CreateFn = unsafe extern "system" fn(
  hinst: HINSTANCE,
  dw_version: u32,
  riidltf: *const GUID,
  ppv_out: *mut *mut c_void,
  punk_outer: *mut IUnknown
) -> HRESULT;

fn main() {
  // DLL name (null-terminated C string)
  let dll_name = CString::new("dinput8.dll").unwrap();

  unsafe {
    let handle = LoadLibraryA(PCSTR(dll_name.as_ptr() as *const u8)).unwrap();

    if handle.0.is_null() {
      panic!("Failed to load DLL.");
    }

    let func_name = CString::new("DirectInput8Create").unwrap();
    let proc = GetProcAddress(handle, PCSTR(func_name.as_ptr() as *const u8));
    if proc.is_none() {
      panic!("Failed to get DirectInput8Create");
    }

    // Cast to a callable function
    let direct_input8_create: DirectInput8CreateFn = std::mem::transmute(proc);

    let mut di_ptr: *mut core::ffi::c_void = std::ptr::null_mut();

    direct_input8_create(
      HINSTANCE(std::ptr::null_mut()),
      DIRECTINPUT_VERSION,
      &IDirectInput8W::IID,
      &mut di_ptr as *mut *mut _,
      std::ptr::null_mut()
    );
  }
}
