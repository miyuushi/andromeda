#[repr(C)]
#[derive(Default)]
pub struct StartupConfig {
  pub process_name: *mut i8,
  pub version: *mut i8
}
