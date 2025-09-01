mod d3d11;
pub(crate) mod win32_backend;

use std::error::Error;

use windows::Win32::Graphics::{
  Direct3D11::{ID3D11RenderTargetView, ID3D11Texture2D},
  Dxgi::IDXGISwapChain
};

use crate::egui_backend::d3d11::dx11_renderer::Dx11Renderer;

pub trait EGuiBackend {
  fn render(&self);
}

#[derive(Default)]
pub struct Viewport {
  swapchain: *mut IDXGISwapChain,
  render_target: *mut ID3D11Texture2D,
  render_target_view: *mut ID3D11RenderTargetView
}

impl Viewport {
  fn new(&self, renderer: &Dx11Renderer, swapchain: *mut IDXGISwapChain, width: u32, height: u32) {}
}

trait Renderer {
  fn create(&self) -> Result<Viewport, Box<dyn Error>>;
}
