use andromeda_common::exports::D3D11CreateDeviceFn;
use log::info;

use crate::{
  D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL, D3D11_CREATE_DEVICE_FLAG, HMODULE, HRESULT, ID3D11Device, ID3D11DeviceContext,
  IDXGIAdapter, entrypoint::D3D11
};

#[unsafe(no_mangle)]
pub unsafe extern "system" fn D3D11CreateDevice(
  p_adapter: *mut IDXGIAdapter,
  driver_type: D3D_DRIVER_TYPE,
  software: HMODULE,
  flags: D3D11_CREATE_DEVICE_FLAG,
  p_feature_levels: *const D3D_FEATURE_LEVEL,
  feature_levels: u32,
  sdk_version: u32,
  pp_device: *mut *mut ID3D11Device,
  p_feature_level: *mut D3D_FEATURE_LEVEL,
  pp_immediate_context: *mut *mut ID3D11DeviceContext
) -> HRESULT {
  info!("D3D11CreateDevice called (feature_levels={feature_levels})");
  let real: D3D11CreateDeviceFn = D3D11.get_orig_fn("D3D11CreateDevice");
  let result = unsafe {
    real(
      p_adapter,
      driver_type,
      software,
      flags,
      p_feature_levels,
      feature_levels,
      sdk_version,
      pp_device,
      p_feature_level,
      pp_immediate_context
    )
  };

  if result.is_ok() && !pp_device.is_null() {
    info!("Got D3D11 device at {:?}", *pp_device);
  }

  result
}
