use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use once_cell::sync::OnceCell;
use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL};
use windows::Win32::Graphics::Direct3D11::{
  D3D11_CREATE_DEVICE_FLAG, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView
};
use windows::Win32::Graphics::Dxgi::{
  DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FULLSCREEN_DESC, IDXGIAdapter, IDXGIFactory,
  IDXGIFactory2, IDXGIOutput, IDXGISwapChain, IDXGISwapChain1
};
use windows::core::{GUID, HRESULT};
use windows_core::IUnknown;

pub(crate) static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);
pub(crate) static EGUI_DX11: Mutex<Option<egui_directx11::Renderer>> = Mutex::new(None);
pub(crate) static RT_VIEW: Mutex<Option<ID3D11RenderTargetView>> = Mutex::new(None);

pub(crate) type PresentFn =
  unsafe extern "system" fn(this: *mut IDXGISwapChain, sync_interval: u32, flags: u32) -> HRESULT;

pub(crate) type CreateSwapChainFn = unsafe extern "system" fn(
  this: *mut IDXGIFactory,
  device: *mut IUnknown,
  desc: *const DXGI_SWAP_CHAIN_DESC,
  swapchain: *mut *mut IDXGISwapChain
) -> HRESULT;

pub(crate) type CreateSwapChainForHwndFn = unsafe extern "system" fn(
  this: *mut IDXGIFactory2,
  device: *mut IUnknown,
  hwnd: HWND,
  desc: *const DXGI_SWAP_CHAIN_DESC1,
  fullscreen_desc: *const DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
  restrict_to_output: *mut IDXGIOutput,
  swapchain: *mut *mut IDXGISwapChain1
) -> HRESULT;

pub(crate) type DXGICreateFactoryFn =
  unsafe extern "system" fn(riid: *const GUID, pp_factory: *mut *mut c_void) -> HRESULT;

pub(crate) type DXGICreateFactory2Fn =
  unsafe extern "system" fn(flags: u32, riid: *const GUID, pp_factory: *mut *mut c_void) -> HRESULT;
