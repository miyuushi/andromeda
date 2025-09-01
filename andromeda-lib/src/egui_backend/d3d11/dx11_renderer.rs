use std::error::Error;

use windows::Win32::{
  Foundation::HWND,
  Graphics::{
    Direct3D11::{ID3D11Device, ID3D11RenderTargetView, ID3D11Texture2D},
    Dxgi::{IDXGIFactory, IDXGISwapChain}
  }
};

use crate::egui_backend::{Renderer, Viewport};

pub struct Dx11Win32Backend {
  egui_renderer: Dx11Renderer,
  swapchain: *mut IDXGISwapChain,
  device: *mut ID3D11Device
}

impl Dx11Win32Backend {
  pub fn new(swapchain: *mut IDXGISwapChain) -> Self {
    Self {
      swapchain,
      egui_renderer: Dx11Renderer::default(),
      device: std::ptr::null_mut() // create egui context here

                                   // self.egui_renderer = Box::new(Dx11Renderer::)
    }
  }
}

#[derive(Default)]
pub struct Dx11Renderer {
  swapchain: *mut IDXGISwapChain,
  hwnd: HWND
}

impl Renderer for Dx11Renderer {
  fn create(&self) -> Result<Viewport, Box<dyn Error>> {
    let mut factory = unsafe { windows::Win32::Graphics::Dxgi::CreateDXGIFactory::<IDXGIFactory>()? };

    // factory.CreateSwapChain(pdevice, pdesc, ppswapchain)

    Ok(Viewport::default())
  }
}
