use windows::{
  Win32::{
    Foundation::HMODULE,
    Graphics::{
      Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL},
      Direct3D11::{D3D11_CREATE_DEVICE_FLAG, ID3D11Device, ID3D11DeviceContext},
      Dxgi::{DXGI_SWAP_CHAIN_DESC, IDXGIAdapter, IDXGISwapChain}
    }
  },
  core::HRESULT
};

pub type D3D11CreateDeviceFn = unsafe extern "system" fn(
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
) -> HRESULT;

pub type D3D11CreateDeviceAndSwapChainFn = unsafe extern "system" fn(
  p_adapter: *mut IDXGIAdapter,
  driver_type: D3D_DRIVER_TYPE,
  software: HMODULE,
  flags: D3D11_CREATE_DEVICE_FLAG,
  p_feature_levels: *const D3D_FEATURE_LEVEL,
  feature_levels: u32,
  sdk_version: u32,
  p_swap_chain_desc: *const DXGI_SWAP_CHAIN_DESC,
  pp_swap_chain: *mut *mut IDXGISwapChain,
  pp_device: *mut *mut ID3D11Device,
  p_feature_level: *mut D3D_FEATURE_LEVEL,
  pp_immediate_context: *mut *mut ID3D11DeviceContext
) -> HRESULT;
