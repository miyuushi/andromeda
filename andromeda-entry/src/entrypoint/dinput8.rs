use std::ffi::c_void;

use log::info;

use crate::{DirectInput8CreateFn, GUID, HINSTANCE, HRESULT, IUnknown, entrypoint::DINPUT8};

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DirectInput8Create(
  hinst: HINSTANCE,
  dw_version: u32,
  riidltf: *const GUID,
  ppv_out: *mut *mut c_void,
  punk_outer: *mut IUnknown
) -> HRESULT {
  info!("DirectInput8Create called (dw_version={dw_version})");
  let real: DirectInput8CreateFn = DINPUT8.get_orig_fn("DirectInput8Create");
  let result = unsafe { real(hinst, dw_version, riidltf, ppv_out, punk_outer) };

  if result.is_ok() && !ppv_out.is_null() {
    info!("Got IDirectInput8 interface at {ppv_out:?}");
  }

  result
}
