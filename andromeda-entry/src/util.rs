use std::{error::Error, mem, os::raw::c_void, slice};

use windows::Win32::{
  Foundation::{GetLastError, HANDLE, HMODULE},
  System::{
    Diagnostics::Debug::{IMAGE_DATA_DIRECTORY, IMAGE_NT_HEADERS32, IMAGE_NT_HEADERS64, IMAGE_NT_OPTIONAL_HDR64_MAGIC},
    Memory::{MEMORY_BASIC_INFORMATION, PAGE_PROTECTION_FLAGS, VirtualProtectEx, VirtualQueryEx},
    ProcessStatus::EnumProcessModules,
    SystemServices::IMAGE_DOS_HEADER,
    Threading::GetCurrentProcess
  }
};

pub mod xiv {
  use std::{fs, path::PathBuf};

  use log::info;

  pub(crate) fn read_game_version(game_path: PathBuf) -> Option<String> {
    let parent = game_path.parent().expect("Could not get parent directory");
    let version_path = parent.join("ffxivgame.ver");
    fs::read_to_string(version_path).ok()
  }
}

pub(crate) struct LoadedModule {
  m_h_module: HMODULE
}

impl LoadedModule {
  pub(crate) fn all_modules() -> Result<Vec<LoadedModule>, Box<dyn Error>> {
    let h_modules: Vec<HMODULE> = Vec::with_capacity(128);

    unsafe {
      let mut dw_needed: Box<u32> = Box::new(0);
      while h_modules.len() < *dw_needed as usize {
        EnumProcessModules(
          GetCurrentProcess(),
          h_modules[0].0 as *mut HMODULE,
          h_modules.as_slice().len() as u32,
          &mut *dw_needed
        )?;
      }

      let mut modules: Vec<LoadedModule> = Vec::with_capacity(h_modules.len());
      for h_module in h_modules {
        if h_module.is_invalid() {
          break;
        }
        modules.push(LoadedModule { m_h_module: h_module });
      }

      Ok(modules)
    }
  }

  fn address(&self, offset: usize) -> *mut c_void {
    unsafe { self.m_h_module.0.add(offset) }
  }
  pub(crate) fn as_ref<T>(&self, offset: usize) -> &T {
    unsafe { &*(self.address(offset) as *const T) }
  }
  fn dos_header(&self) -> &IMAGE_DOS_HEADER {
    self.as_ref::<IMAGE_DOS_HEADER>(0)
  }
  fn nt_header32(&self) -> &IMAGE_NT_HEADERS32 {
    self.as_ref::<IMAGE_NT_HEADERS32>(self.dos_header().e_lfanew as usize)
  }
  fn nt_header64(&self) -> &IMAGE_NT_HEADERS64 {
    self.as_ref::<IMAGE_NT_HEADERS64>(self.dos_header().e_lfanew as usize)
  }
  fn is_pe64(&self) -> bool {
    self.nt_header32().OptionalHeader.Magic == IMAGE_NT_OPTIONAL_HDR64_MAGIC
  }

  fn data_directories(&self) -> &[IMAGE_DATA_DIRECTORY] {
    if self.is_pe64() {
      &self.nt_header64().OptionalHeader.DataDirectory
    } else {
      &self.nt_header32().OptionalHeader.DataDirectory
    }
  }
  pub(crate) fn data_directory(&self, index: usize) -> &IMAGE_DATA_DIRECTORY {
    &self.data_directories()[index]
  }
}

pub struct MemoryTenderizer;

impl MemoryTenderizer {
  pub fn init(
    p_address: &[std::os::raw::c_char; 1],
    length: usize,
    dw_new_protect: PAGE_PROTECTION_FLAGS
  ) -> std::io::Result<()> {
    Self::init_with_process(unsafe { GetCurrentProcess() }, p_address, length, dw_new_protect)
  }

  pub fn init_with_process(
    h_process: HANDLE,
    p_address: &[std::os::raw::c_char; 1],
    length: usize,
    dw_new_protect: PAGE_PROTECTION_FLAGS
  ) -> std::io::Result<()> {
    let data = unsafe { slice::from_raw_parts(p_address.as_ptr() as *const u8, length) };
    Self::change_protection(h_process, data, dw_new_protect)
  }

  fn change_protection(h_process: HANDLE, m_data: &[u8], dw_new_protect: PAGE_PROTECTION_FLAGS) -> std::io::Result<()> {
    // collected regions so we can restore on error
    let mut regions: Vec<MEMORY_BASIC_INFORMATION> = Vec::new();

    // helper to restore protections (called on error)
    let restore_on_error = |regions: &mut Vec<MEMORY_BASIC_INFORMATION>| {
      for region in regions.iter().rev() {
        // old_protect placeholder
        let mut _old: PAGE_PROTECTION_FLAGS = PAGE_PROTECTION_FLAGS(0);
        let ok = unsafe {
          VirtualProtectEx(
            h_process,
            region.BaseAddress,
            region.RegionSize,
            region.Protect,
            &mut _old
          )
        }
        .is_ok();
        if !ok {
          // Use abort() to emulate immediate termination
          eprintln!(
            "Failed to restore protection for region {:p} size {:#X}; last_error={}",
            region.BaseAddress,
            region.RegionSize,
            unsafe { GetLastError().0 }
          );
          std::process::abort();
        }
      }
    };

    unsafe {
      let start = m_data.as_ptr();
      let end = start.add(m_data.len());
      // p_covered initially points to start of m_data
      let mut p_covered = start as *const u8;

      while (p_covered as *const u8) < end {
        // prepare an empty MEMORY_BASIC_INFORMATION
        let mut mbi: MEMORY_BASIC_INFORMATION = mem::zeroed();

        // VirtualQueryEx returns the number of bytes returned (0 on failure)
        let ret = VirtualQueryEx(
          h_process,
          Some(p_covered as *const _),
          &mut mbi as *mut MEMORY_BASIC_INFORMATION,
          mem::size_of::<MEMORY_BASIC_INFORMATION>()
        );

        if ret == 0 {
          let err = std::io::Error::from_raw_os_error(GetLastError().0 as i32);
          // restore and return error
          restore_on_error(&mut regions);
          return Err(std::io::Error::new(
            err.kind(),
            format!(
              "VirtualQueryEx(addr={:p}, cb={}) failed: {}",
              p_covered,
              mem::size_of::<MEMORY_BASIC_INFORMATION>(),
              err
            )
          ));
        }

        // Call VirtualProtectEx to change protection, saving old protection in old_protect
        let mut old_protect: u32 = 0;
        let ok = VirtualProtectEx(
          h_process,
          mbi.BaseAddress,
          mbi.RegionSize,
          dw_new_protect,
          &mut mbi.Protect
        )
        .is_ok();

        if !ok {
          let err = std::io::Error::from_raw_os_error(GetLastError().0 as i32);
          // restore and return error
          restore_on_error(&mut regions);
          return Err(std::io::Error::new(
            err.kind(),
            format!(
              "VirtualProtectEx(addr={:p}, size={:#X}) failed: {}",
              mbi.BaseAddress, mbi.RegionSize, err
            )
          ));
        }

        // Note: keep the original Protect value so we can restore later
        // Some definitions of MEMORY_BASIC_INFORMATION use `Protect` as the current protection field.
        // We push the returned `mbi` (which contains .Protect) to the vector.
        regions.push(mbi);

        // Advance p_covered to end of the range we just processed:
        // region end = BaseAddress + RegionSize
        p_covered = (mbi.BaseAddress as *const u8).add(mbi.RegionSize as usize);
      }

      // success — on normal exit we don't need to restore anything (the protections remain changed)
      Ok(())
    }
  }
}
