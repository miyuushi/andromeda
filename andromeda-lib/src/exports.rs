use std::ffi::c_void;

use once_cell::sync::OnceCell;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::{GUID, HRESULT};

use crate::util::log;

pub(crate) type PresentFn =
  unsafe extern "system" fn(this: *mut IDXGISwapChain, sync_interval: u32, flags: u32) -> HRESULT;

static ORIG_PRESENT: OnceCell<PresentFn> = OnceCell::new();

pub(crate) type DXGICreateFactoryFn =
  unsafe extern "system" fn(riid: *const GUID, pp_factory: *mut *mut c_void) -> HRESULT;

pub(crate) static ORIG_DXGI_CREATE_FACTORY: OnceCell<DXGICreateFactoryFn> = OnceCell::new();

pub(crate) unsafe extern "system" fn hooked_dxgi_create_factory(
  riid: *const GUID,
  pp_factory: *mut *mut c_void
) -> HRESULT {
  log("[Andromeda] hooked DXGICreateFactory called");

  let orig = ORIG_DXGI_CREATE_FACTORY.get().unwrap();
  let hr = unsafe { orig(riid, pp_factory) };

  if hr.is_ok() && !pp_factory.is_null() {
    log("[Andromeda] Factory created - waiting for swapchain");
  }

  hr
}
