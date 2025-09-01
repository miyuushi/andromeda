use std::{
  any::Any,
  ffi::c_void,
  rc::Rc,
  sync::{Arc, Mutex, Weak}
};

use once_cell::sync::OnceCell;
use windows::Win32::Graphics::{
  Direct3D11::{ID3D11Device, ID3D11DeviceContext},
  Dxgi::IDXGISwapChain
};
use windows_core::{HRESULT, Interface};

use crate::exports::PresentFn;

// static mut GAME_DEVICE_SWAPCHAIN: Option<Box<dyn SwapchainBase + Send>> = None;

// I guess we assume the game device swapchain is the first one we find, which is often the case, but this should be handled better
pub fn game_device_swapchain() -> Option<Arc<dyn SwapchainBase + Send + Sync>> {
  if let Some(render_target) = RENDER_TARGETS.get() &&
    let Ok(mutex) = render_target.lock()
  {
    return (*mutex).first().cloned();
  }
  None
}

// ---------------- Backend Abstraction ----------------
#[derive(Clone, Copy, Debug)]
pub enum Backend {
  DX11,
  Vulkan
}

pub trait SwapchainBase {
  fn backend(&self) -> Backend;
  fn present(&self, sync_interval: u32, flags: u32);

  fn as_any(&self) -> &dyn Any;
}

pub static RENDER_TARGETS: OnceCell<Mutex<Vec<Arc<dyn SwapchainBase + Send + Sync>>>> =
  OnceCell::with_value(Mutex::new(Vec::new()));

pub fn register_swapchain(s: Arc<dyn SwapchainBase + Send + Sync>) {
  RENDER_TARGETS.get().unwrap().lock().unwrap().push(s);
}

// ---------------- DX11 Swapchain Wrapper ----------------
#[derive(Clone)]
pub struct DX11Swapchain {
  pub swapchain: IDXGISwapChain,
  pub device: ID3D11Device,
  pub context: ID3D11DeviceContext,
  pub original_present: Option<PresentFn>
}

impl SwapchainBase for DX11Swapchain {
  fn backend(&self) -> Backend {
    Backend::DX11
  }

  fn present(&self, sync_interval: u32, flags: u32) {
    unsafe {
      if let Some(orig) = self.original_present {
        orig(&self.swapchain as *const _ as *mut _, sync_interval, flags);
      }
    }
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl DX11Swapchain {
  // pub unsafe fn hook_present(&mut self) -> Result<(), &'static str> {
  //   let vtable = *(self.swapchain.as_raw() as *mut *mut *mut c_void);
  //   let present_addr = *vtable.add(8);

  //   self.original_present = Some(std::mem::transmute(
  //     min_hook_rs::create_hook(
  //       present_addr as *mut c_void,
  //       dx11_present_hook as *mut c_void
  //     )
  //     .map_err(|_| "Failed to create Present hook")?
  //   ));

  //   min_hook_rs::enable_hook(present_addr as *mut c_void)
  //     .map_err(|_| "Failed to enable Present hook")?;
  //   Ok(())
  // }
}

// ---------------- Vulkan Swapchain Wrapper ----------------
#[derive(Clone)]
pub struct VulkanSwapchain {
  pub swapchain: ash::vk::SwapchainKHR,
  pub device: ash::Device,
  pub queue: ash::vk::Queue
}

impl SwapchainBase for VulkanSwapchain {
  fn backend(&self) -> Backend {
    Backend::Vulkan
  }

  fn present(&self, _sync_interval: u32, _flags: u32) {
    // Submit overlay command buffers and call vkQueuePresentKHR
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

// ---------------- Utilities ----------------
// pub trait AsAny {
//   fn as_any(&self) -> &dyn std::any::Any;
// }

// impl<T: 'static> AsAny for T {
//     fn as_any(&self) -> &dyn std::any::Any { self }
// }

impl SwapchainBase for Arc<dyn SwapchainBase + Send + Sync> {
  fn backend(&self) -> Backend {
    (**self).backend()
  }
  fn present(&self, sync_interval: u32, flags: u32) {
    (**self).present(sync_interval, flags)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

// impl<T: SwapchainBase + Send + 'static> AsAny for T {
//   fn as_any(&self) -> &dyn std::any::Any {
//     self
//   }
// }
