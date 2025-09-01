use andromeda_common::exports::{D3D11CreateDeviceAndSwapChainFn, D3D11CreateDeviceFn};
use log::{error, info};
use once_cell::sync::{Lazy, OnceCell};
use std::{
  cell::Cell,
  collections::HashSet,
  ffi::c_void,
  ptr::{NonNull, swap},
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicPtr, Ordering}
  }
};
use windows::{
  Win32::{
    Foundation::{HINSTANCE, HMODULE, HWND},
    Graphics::{
      Direct3D::{
        D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_REFERENCE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0
      },
      Direct3D11::{D3D11_CREATE_DEVICE_FLAG, ID3D11Device, ID3D11DeviceContext},
      Dxgi::{
        DXGI_ADAPTER_DESC, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FULLSCREEN_DESC, IDXGIAdapter,
        IDXGIDevice, IDXGIDevice1, IDXGIFactory, IDXGIFactory1, IDXGIFactory2, IDXGIOutput, IDXGISwapChain,
        IDXGISwapChain1
      }
    }
  },
  core::{GUID, HRESULT}
};
use windows_core::{IUnknown, Interface};

use crate::{
  exports::{CreateSwapChainFn, CreateSwapChainForHwndFn, DXGICreateFactory2Fn, DXGICreateFactoryFn, PresentFn},
  hooks::hook_vtable_method,
  internal::{
    INTERFACES,
    swapchain_util::{DX11Swapchain, RENDER_TARGETS, register_swapchain}
  },
  util::{get_module_symbol_address, hresult_to_string, log}
};

pub(crate) static ORIG_CREATE_SWAPCHAIN: OnceLock<CreateSwapChainFn> = OnceLock::new();
pub(crate) static ORIG_CREATE_SWAPCHAIN_FOR_HWND: OnceLock<CreateSwapChainForHwndFn> = OnceLock::new();
// pub(crate) static ORIG_CREATE_SWAPCHAIN_FOR_COREWINDOW: OnceCell<CreateSwapChainForCoreWindowFn> =
//   OnceCell::new();
// pub(crate) static ORIG_CREATE_SWAPCHAIN_FOR_COMPOSITION: OnceCell<CreateSwapChainForCompositionFn> =
//   OnceCell::new();
pub struct DX11Hooks {
  pub d3d11_create_device: OnceLock<D3D11CreateDeviceFn>,
  pub d3d11_create_device_and_sc: OnceLock<D3D11CreateDeviceAndSwapChainFn>,
  pub dxgi_create_factory: OnceLock<DXGICreateFactoryFn>,
  pub dxgi_create_factory1: OnceLock<DXGICreateFactoryFn>,
  pub dxgi_create_factory2: OnceLock<DXGICreateFactory2Fn>
}

impl DX11Hooks {
  pub const fn new() -> Self {
    Self {
      d3d11_create_device: OnceLock::new(),
      d3d11_create_device_and_sc: OnceLock::new(),
      dxgi_create_factory: OnceLock::new(),
      dxgi_create_factory1: OnceLock::new(),
      dxgi_create_factory2: OnceLock::new()
    }
  }
}

pub(crate) static DX11_HOOKS: DX11Hooks = DX11Hooks::new();

static HOOKED_FACTORIES: Lazy<Mutex<HashSet<usize>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static HOOKED_PRESENT_SLOTS: Lazy<Mutex<HashSet<usize>>> = Lazy::new(|| Mutex::new(HashSet::new()));

thread_local! {
  static G_IN_DXGI_RUNTIME: Cell<bool> = const { Cell::new(false) }
}

pub(crate) unsafe extern "system" fn d3d11_create_device_hook(
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
  info!("[HOOK] Hooked D3D11CreateDevice called");

  // if G_IN_DXGI_RUNTIME.get() {
  // Forward to original
  let orig = DX11_HOOKS
    .d3d11_create_device
    .get()
    .expect("orig D3D11CreateDevice missing");
  return orig(
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
  );
  // }

  // hooked_d3d11_create_device_and_swapchain(
  //   p_adapter,
  //   driver_type,
  //   software,
  //   flags,
  //   p_feature_levels,
  //   feature_levels,
  //   sdk_version,
  //   std::ptr::null(),
  //   OnceLock::new(),
  //   pp_device,
  //   p_feature_level,
  //   pp_immediate_context
  // )
}

pub(crate) unsafe extern "system" fn d3d11_create_device_and_swapchain_hook(
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
) -> HRESULT {
  info!("[HOOK] Hooked D3D11CreateDeviceAndSwapChain called");
  let orig = DX11_HOOKS
    .d3d11_create_device_and_sc
    .get()
    .expect("orig D3D11CreateDeviceAndSwapChain missing");

  // windows::Win32::Graphics::Direct3D11::D3D11CreateDeviceAndSwapChain()

  if G_IN_DXGI_RUNTIME.get() {
    // Forward to original
    return orig(
      p_adapter,
      driver_type,
      software,
      flags,
      p_feature_levels,
      feature_levels,
      sdk_version,
      p_swap_chain_desc,
      pp_swap_chain,
      pp_device,
      p_feature_level,
      pp_immediate_context
    );
  }

  let feature_level = D3D_FEATURE_LEVEL_11_0;

  G_IN_DXGI_RUNTIME.set(true);
  let mut hr = orig(
    p_adapter,
    driver_type,
    software,
    flags,
    p_feature_levels,
    feature_levels,
    sdk_version,
    std::ptr::null_mut(),
    std::ptr::null_mut(),
    pp_device,
    p_feature_level,
    std::ptr::null_mut()
  );
  G_IN_DXGI_RUNTIME.set(false);

  if hr.is_err() {
    info!(
      "[HOOK] D3D11CreateDeviceAndSwapChain failed with error code {}",
      hresult_to_string(hr)
    );
    return hr;
  }

  if !p_feature_level.is_null() {
    *p_feature_level = feature_level;
  }

  info!("Using feature level {}", feature_level.0);

  if pp_device.is_null() {
    assert!(pp_swap_chain.is_null() && pp_immediate_context.is_null());
    return hr;
  }

  let device = unsafe { ID3D11Device::from_raw(*pp_device as *mut c_void) };
  let dxgi_device: IDXGIDevice1 = device
    .cast()
    .map_err(|e| hr = e.code())
    .expect("Failed to obtain IDXGIDevice!");

  let mut adapter: Option<IDXGIAdapter> = match p_adapter.is_null() {
    true => None,
    false => Some(IDXGIAdapter::from_raw(p_adapter as *mut c_void))
  };

  // let device_proxy: *mut D3D11Device;
  if driver_type == D3D_DRIVER_TYPE_WARP || driver_type == D3D_DRIVER_TYPE_REFERENCE {
    info!(
      "Skipping device because the driver Calling IDXGIFactory::CreateSwapChain:type is 'D3D_DRIVER_TYPE_WARP' or 'D3D_DRIVER_TYPE_REFERENCE'"
    );
  } else if !p_adapter.is_null() && let Some(ref adapter) = adapter && let Ok(adapter_desc) = unsafe { adapter.GetDesc() } &&
      adapter_desc.VendorId == 0x1414 /* Microsoft */ && adapter_desc.DeviceId == 0x8C
  /* Microsoft Basic Render Driver */
  {
    info!("Skipping device because it uses the Microsoft Basic Render Driver");
  } else if let Ok(device_context) = unsafe { device.GetImmediateContext() } {
    // device, device_proxy = D3D11Device::new(dxgi_device, device);
    // device_proxy.immediate_context = D3D11DeviceContext::new()
  }

  if !p_swap_chain_desc.is_null() {
    assert!(!pp_swap_chain.is_null());

    if adapter.is_none() {
      adapter = match unsafe { dxgi_device.GetAdapter() } {
        Ok(a) => Some(a),
        Err(e) => {
          hr = e.code();
          None
        }
      };
      assert!(hr.is_ok());
    }

    let mut factory: Option<IDXGIFactory> = None;
    if let Some(ref adapter) = adapter {
      match unsafe { adapter.GetParent() } {
        Ok(f) => factory = Some(f),
        Err(e) => hr = e.code()
      }
      info!("nice");
      assert!(hr.is_ok());
    }

    info!("[HOOK] Calling IDXGIFactory::CreateSwapChain:");
    if let Some(factory) = factory {
      hr = create_swapchain_hook(
        factory.as_raw() as *mut IDXGIFactory,
        device.as_raw() as *mut IUnknown,
        p_swap_chain_desc,
        pp_swap_chain
      );
    }
    // hr = create_swapchain_hook(factory as *mut IDXGIFactory, device as *mut c_void, p_swap_chain_desc as *mut c_void, pp_swap_chain);
  }

  if hr.is_ok() {
    info!("nice");
    if !pp_immediate_context.is_null() {
      unsafe { *pp_immediate_context = device.GetImmediateContext().unwrap().as_raw() as *mut ID3D11DeviceContext };
    }
  } else {
    unsafe { *pp_device = std::ptr::null_mut() };
  }

  // if hr.is_ok() {
  //   let swapchain = IDXGISwapChain::from_raw(*pp_swap_chain);
  //   let device = ID3D11Device::from_raw(*pp_device);
  //   let _ = hook_swapchain(swapchain, device);
  // }

  hr
}

// ----------------- Hook Present -----------------
pub(crate) unsafe extern "system" fn dxgi_present_hook(
  this: *mut IDXGISwapChain,
  sync_interval: u32,
  flags: u32
) -> HRESULT {
  if let Ok(mut i) = INTERFACES.get().unwrap().lock() {
    (*i).render_andromeda();
  }

  let swapchain = this as *mut c_void;

  if let Some(hooked) = RENDER_TARGETS
    .get()
    .expect("Failed to retrieve render targets")
    .lock()
    .unwrap()
    .iter()
    .find_map(|h| {
      h.as_any()
        .downcast_ref::<DX11Swapchain>()
        .filter(|dx11| dx11.swapchain.as_raw() == swapchain)
    })
  {
    if let Some(orig) = hooked.original_present {
      return orig(this, sync_interval, flags);
    }
  }

  HRESULT(0)
}

unsafe fn install_dxgi_present_hook(
  swapchain: IDXGISwapChain,
  device: ID3D11Device
) -> Result<PresentFn, &'static str> {
  // Get immediate context
  let context = device
    .GetImmediateContext()
    .map_err(|_| "Failed to get immediate context")?;

  // Get Present vtable
  let vtable = *(swapchain.as_raw() as *mut *mut *mut c_void);
  let present_addr = *vtable.add(8);

  if HOOKED_PRESENT_SLOTS.lock().unwrap().contains(&(present_addr as usize)) {
    return Err("Present vtable already hooked");
  }

  let original_present: PresentFn = std::mem::transmute(
    min_hook_rs::create_hook(present_addr as *mut _, dxgi_present_hook as *mut _)
      .map_err(|_| "Failed to create Present hook")?
  );

  info!("My swapchain pointer is {:?}", swapchain.as_raw());

  // Push to renderer-independent registry
  register_swapchain(Arc::new(DX11Swapchain {
    swapchain: swapchain.clone(),
    device: device.clone(),
    context: context.clone(),
    original_present: Some(original_present)
  }));

  HOOKED_PRESENT_SLOTS.lock().unwrap().insert(present_addr as usize);

  min_hook_rs::enable_hook(present_addr as *mut _).map_err(|_| "Failed to enable Present hook")?;

  // HOOKED_SWAPCHAINS.lock().unwrap().push(DX11Swapchain {
  //   swapchain,
  //   device,
  //   context,
  //   original_present: Some(original_present)
  // });

  Ok(original_present)
}

unsafe fn on_swapchain_created(swapchain: *mut IDXGISwapChain, device: *mut ID3D11Device) -> Result<(), String> {
  if !swapchain.is_null() && !device.is_null() {
    let swapchain = swapchain as *mut _;
    let sc = IDXGISwapChain::from_raw(swapchain);
    let device = device as *mut _;
    let device = ID3D11Device::from_raw(device);
    // if let Some(sc) = sc
    //   && let Some(device) = device
    // {
    info!("[HOOK] Swapchain created: {:?}", swapchain);
    install_dxgi_present_hook(sc, device)?;
    // }
  }
  Err("There was an error when creating a swapchain".into())
}

// IDXGIFactory::CreateSwapChain
pub(crate) unsafe extern "system" fn create_swapchain_hook(
  this: *mut IDXGIFactory,
  device: *mut IUnknown,
  desc: *const DXGI_SWAP_CHAIN_DESC,
  swapchain: *mut *mut IDXGISwapChain
) -> HRESULT {
  info!("[HOOK] hooked create_swapchain called");

  // call original CreateSwapChain
  let orig = ORIG_CREATE_SWAPCHAIN.get().expect("orig CreateSwapChain missing");
  let hr = unsafe { orig(this, device, desc, swapchain) };

  if hr.is_ok() && !swapchain.is_null() {
    info!("swapchain is created");
    let sc = unsafe { *swapchain };
    on_swapchain_created(sc, device as *mut ID3D11Device);
  }
  hr
}

pub(crate) unsafe extern "system" fn create_swapchain_for_hwnd_hook(
  this: *mut IDXGIFactory2,
  device: *mut IUnknown,
  hwnd: HWND,
  desc: *const DXGI_SWAP_CHAIN_DESC1,
  fullscreen_desc: *const DXGI_SWAP_CHAIN_FULLSCREEN_DESC,
  restrict_to_output: *mut IDXGIOutput,
  swapchain: *mut *mut IDXGISwapChain1
) -> HRESULT {
  unsafe {
    let orig = ORIG_CREATE_SWAPCHAIN_FOR_HWND
      .get()
      .expect("orig CreateSwapChainForHwnd missing");
    let hr = orig(this, device, hwnd, desc, fullscreen_desc, restrict_to_output, swapchain);
    if hr.is_ok() {
      info!("eeee");
    }
    hr
  }
}

/// Marks a factory vtable as hooked; returns true if this is a new vtable
unsafe fn mark_dxgi_factory_hooked(factory_ptr: *mut IDXGIFactory) -> bool {
  let vtable = *(factory_ptr as *mut *mut *mut c_void) as usize;
  let mut hooked = HOOKED_FACTORIES.lock().unwrap();
  hooked.insert(vtable) // true if newly inserted
}

/// Called once when you intercept CreateDXGIFactory[1/2]
unsafe fn install_dxgi_factory_hooks(factory_ptr: *mut IDXGIFactory) -> Result<(), String> {
  if !mark_dxgi_factory_hooked(factory_ptr) {
    return Err("Factory vtable already hooked".into());
  }

  let factory_ptr = factory_ptr as *mut c_void;
  let factory = IDXGIFactory::from_raw_borrowed(&factory_ptr).ok_or("Failed to borrow DXGIFactory")?;
  let vtable = *(factory_ptr as *mut *mut *mut c_void);
  info!("Factory vtable: {:?}", *vtable);

  // Always hook IDXGIFactory::CreateSwapChain (index 10)
  if let Ok(trampoline) = hook_vtable_method(vtable, 10, create_swapchain_hook as *mut c_void) &&
    ORIG_CREATE_SWAPCHAIN.get().is_none()
  {
    ORIG_CREATE_SWAPCHAIN.set(std::mem::transmute(trampoline));
    info!("hooked createswapchain");
  }

  // Try casting to newer factories and hook more if available
  if let Ok(factory1) = factory.cast::<IDXGIFactory1>() {
    info!("Factory supports IDXGIFactory1");
    // Nothing new to hook here, but confirms runtime version
  }

  if let Ok(factory2) = factory.cast::<IDXGIFactory2>() {
    let vtable = *(factory2.as_raw() as *mut *mut *mut c_void);
    info!("Factory supports IDXGIFactory2");

    // IDXGIFactory2::CreateSwapChainForHwnd (index 15)
    if let Ok(trampoline) = hook_vtable_method(vtable, 15, create_swapchain_for_hwnd_hook as *mut c_void) &&
      ORIG_CREATE_SWAPCHAIN_FOR_HWND.get().is_none()
    {
      ORIG_CREATE_SWAPCHAIN_FOR_HWND.set(std::mem::transmute(trampoline));
    }

    // // IDXGIFactory2::CreateSwapChainForCoreWindow (index 16)
    // hook_vtable_method(
    //   vtable,
    //   16,
    //   create_swapchain_for_corewindow_detour as *mut c_void
    // )?;

    // // IDXGIFactory2::CreateSwapChainForComposition (index 24)
    // hook_method(
    //   vtable,
    //   24,
    //   create_swapchain_for_composition_detour as *mut c_void
    // )?;
  }

  Ok(())
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "system" fn dxgi_create_factory_hook(
  riid: *const GUID,
  pp_factory: *mut *mut c_void
) -> HRESULT {
  info!("[HOOK] hooked DXGICreateFactory called");

  let orig = DX11_HOOKS
    .dxgi_create_factory
    .get()
    .expect("orig CreateFactory not found");
  let hr = unsafe { orig(riid, pp_factory) };

  if hr.is_ok() && !pp_factory.is_null() {
    let factory = *pp_factory as *mut IDXGIFactory;
    info!("[HOOK] Factory created - waiting for swapchain");
    match unsafe { install_dxgi_factory_hooks(factory) } {
      Ok(_) => info!("[HOOK] Installed DXGIFactory hooks successfully!"),
      Err(e) => error!("Error installing DXGIFactory hooks: {}", e)
    }
  }

  hr
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "system" fn dxgi_create_factory1_hook(
  riid: *const GUID,
  pp_factory: *mut *mut c_void
) -> HRESULT {
  info!("[HOOK] hooked DXGICreateFactory1 called");

  let orig = DX11_HOOKS
    .dxgi_create_factory1
    .get()
    .expect("orig CreateFactory1 not found");
  let hr = unsafe { orig(riid, pp_factory) };

  if hr.is_ok() && !pp_factory.is_null() {
    let factory = *pp_factory as *mut IDXGIFactory;
    info!("[HOOK] Factory created - waiting for swapchain");
    match unsafe { install_dxgi_factory_hooks(factory) } {
      Ok(_) => info!("[HOOK] Installed DXGIFactory hooks successfully!"),
      Err(e) => error!("Error installing DXGIFactory hooks: {}", e)
    }
  }

  hr
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "system" fn dxgi_create_factory2_hook(
  flags: u32,
  riid: *const GUID,
  pp_factory: *mut *mut c_void
) -> HRESULT {
  info!("[HOOK] hooked DXGICreateFactory2 called");

  let orig = DX11_HOOKS
    .dxgi_create_factory2
    .get()
    .expect("orig CreateFactory2 not found");
  let hr = unsafe { orig(flags, riid, pp_factory) };

  if hr.is_ok() && !pp_factory.is_null() {
    let factory = *pp_factory as *mut IDXGIFactory;
    info!("[HOOK] Factory created - waiting for swapchain");
    match unsafe { install_dxgi_factory_hooks(factory) } {
      Ok(_) => info!("[HOOK] Installed DXGIFactory hooks successfully!"),
      Err(e) => error!("Error installing DXGIFactory hooks: {}", e)
    }
  }

  hr
}
