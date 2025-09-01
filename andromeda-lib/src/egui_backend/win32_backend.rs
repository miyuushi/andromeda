use crate::egui_backend::EGuiBackend;

#[derive(Default)]
pub(crate) struct Win32Backend;

impl EGuiBackend for Win32Backend {
  fn render(&self) {}
}
