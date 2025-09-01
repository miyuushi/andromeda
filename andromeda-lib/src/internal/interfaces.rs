use std::{ptr::swap, sync::Arc};

use windows::{
  Win32::Graphics::{
    Direct3D11::{ID3D11Device, ID3D11RenderTargetView, ID3D11Texture2D},
    Dxgi::IDXGISwapChain
  },
  core::HRESULT
};

use crate::{
  egui_backend::{EGuiBackend, win32_backend::Win32Backend},
  exports::{EGUI_CTX, EGUI_DX11, RT_VIEW},
  internal::swapchain_util::{Backend, DX11Swapchain, RENDER_TARGETS, game_device_swapchain}
};

// if let (Some(ctx), Some(painter)) = (&EGUI_CTX, &mut EGUI_DX11) {
//   let sc = &*swapchain;

//   // Get backbuffer
//   if RT_VIEW.is_none() {
//     let mut desc = sc.GetDesc().unwrap_or_default();
//     let mut bb: Option<ID3D11Texture2D> = None;

//     sc.GetBuffer::<ID3D11Texture2D>(0).unwrap();

//     let device: ID3D11Device = sc.GetDevice::<ID3D11Device>().unwrap();

//     let mut rtv: Option<ID3D11RenderTargetView> = None;
//     device.CreateRenderTargetView(&bb.unwrap(), None, Some(&mut rtv)).unwrap();
//     RT_VIEW = rtv;
//   }

//   // Begin egui frame
//   let raw_input = egui::RawInput::default();
//   ctx.begin_pass(raw_input);

//   egui::Window::new("Overlay").show(ctx, |ui| {
//       ui.label("Hello from egui!");
//   });

//   let full_output = ctx.end_pass();
//   let paint_jobs = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

//   let tex_delta = full_output.textures_delta;
//   painter.render(&ctx, RT_VIEW, &ctx).unwrap();
// }

pub(crate) struct Interfaces {
  backend: Option<Arc<dyn EGuiBackend>>
}

unsafe impl Send for Interfaces {}
unsafe impl Sync for Interfaces {}

impl Interfaces {
  pub fn new() -> Self {
    Self { backend: None }
  }

  unsafe fn setup_hooks() {}

  pub unsafe fn render_andromeda(&mut self) -> Option<Arc<dyn EGuiBackend>> {
    let backend = unsafe { self.backend.clone().or_else(|| self.init_backend()) };

    if let Ok(mut c) = EGUI_CTX.lock() {
      if c.is_none() {
        *c = Some(egui::Context::default());
      }
    }

    if let Ok(mut r) = EGUI_DX11.lock() {
      let game_device = game_device_swapchain().unwrap();
      let game_device = game_device.as_any().downcast_ref::<DX11Swapchain>().unwrap();
      if r.is_none() {
        let swap_chain_texture = game_device.swapchain.GetBuffer::<ID3D11Texture2D>(0).unwrap();
        if let Ok(mut rt) = RT_VIEW.lock() {
          if rt.is_none() {
            let mut render_target = None;
            game_device
              .device
              .CreateRenderTargetView(&swap_chain_texture, None, Some(&mut render_target));
            *rt = render_target;
          }
        }
        *r = Some(egui_directx11::Renderer::new(&game_device.device).unwrap());
      }
    }

    let targets = RENDER_TARGETS.get().unwrap().lock().unwrap();
    for t in targets.iter() {
      match t.backend() {
        Backend::DX11 => {
          let context = t.as_any().downcast_ref::<DX11Swapchain>().unwrap();
          if let (Ok(ctx), Ok(mut renderer), Ok(mut rt)) = (EGUI_CTX.lock(), EGUI_DX11.lock(), RT_VIEW.lock()) {
            if let Some(egui_ctx) = ctx.as_ref() &&
              let Some(egui_dx11) = renderer.as_mut() &&
              let Some(rt) = rt.as_mut()
            {
              // Begin egui frame
              let raw_input = egui::RawInput::default();
              egui_ctx.begin_pass(raw_input);

              egui::Window::new("Andromeda")
                .default_pos(egui::pos2(500.0, 300.0))
                .default_size(egui::Vec2::new(800.0, 500.0))
                .show(&egui_ctx, |ui| {
                  ui.label("Hello from egui!");
                  ui.add(egui::Slider::new(&mut 25, 0..=100).text("Sliiide"));

                  if ui.button("Testing testing").clicked() {}
                });

              let full_output = egui_ctx.end_pass();
              let (renderer_output, platform_output, _) = egui_directx11::split_output(full_output);
              egui_dx11.render(&context.context, &rt, &egui_ctx, renderer_output, 1.0);
            }
          }
        }
        Backend::Vulkan => {
          // Call Vulkan overlay renderer
        }
      }
    }

    backend
  }

  unsafe fn init_backend(&mut self) -> Option<Arc<dyn EGuiBackend>> {
    let backend: Arc<Win32Backend> = Arc::new(Default::default());

    self.backend = Some(backend.clone());
    Some(backend)
  }
}
