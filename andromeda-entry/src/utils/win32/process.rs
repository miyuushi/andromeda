use std::{
  ffi::OsString,
  path::{Path, PathBuf}
};

use windows::{
  Win32::{
    Foundation::{HANDLE, MAX_PATH},
    System::{
      Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
      Memory::{
        MEM_COMMIT, MEM_IMAGE, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, VIRTUAL_ALLOCATION_TYPE,
        VirtualAllocEx, VirtualQueryEx
      },
      ProcessStatus::GetMappedFileNameW,
      Threading::{GetCurrentProcess, PROCESS_NAME_WIN32, QueryFullProcessImageNameW}
    },
    UI::{Shell::PATHCCH_MAX_CCH, WindowsAndMessaging::CharLowerW}
  },
  core::PWSTR
};

use std::os::windows::ffi::OsStringExt;

pub(crate) struct Process {
  handle: HANDLE
}

impl Process {
  pub fn new(handle: HANDLE) -> Self {
    Self { handle }
  }

  pub fn current() -> Self {
    Self {
      handle: unsafe { GetCurrentProcess() }
    }
  }

  /// Get the full path of the process executable
  pub fn path_of(&self) -> Option<PathBuf> {
    let mut buffer = vec![0u16; MAX_PATH as usize];
    let mut size = buffer.len() as u32;

    unsafe {
      QueryFullProcessImageNameW(
        self.handle,
        PROCESS_NAME_WIN32,
        PWSTR::from_raw(buffer.as_mut_ptr()),
        &mut size
      )
      .ok()?
    }

    buffer.truncate(size as usize);
    Some(PathBuf::from(OsString::from_wide(&buffer)))
  }

  /// Get just the filename of the executable
  pub fn base_name(&self) -> Option<PathBuf> {
    self.path_of().and_then(|p| p.file_name().map(PathBuf::from))
  }

  /// Lowercase a UTF-16 string in place (equivalent to CharLowerW)
  pub fn lowercase_utf16(s: &mut [u16]) {
    unsafe { CharLowerW(PWSTR::from_raw(s.as_mut_ptr())) };
  }

  fn get_mapped_image_native_path(handle: HANDLE, lp_mem: *mut u8) -> anyhow::Result<PathBuf> {
    // Allocate buffer for WCHARs
    let mut buffer: Vec<u16> = vec![0u16; PATHCCH_MAX_CCH as usize];

    let len = unsafe { GetMappedFileNameW(handle, lp_mem as *mut _, &mut buffer) } as usize;

    if len == 0 {
      anyhow::bail!("GetMappedFileNameW failed");
    }

    buffer.truncate(len); // remove unused elements
    let result =
      String::from_utf16(&buffer).map_err(|_| anyhow::anyhow!("Failed to convert WCHAR to UTF-16 string"))?;

    // Handle \Device\ prefix
    if let Some(rest) = result.strip_prefix(r"\Device\") {
      let path = format!(r"\\?\{}", &rest[0..]);
      Ok(PathBuf::from(path))
    } else {
      anyhow::bail!("Path unprocessable: {}", result);
    }
  }

  pub fn get_committed_image_allocation_with_path(&self, path: &Path) -> Vec<MEMORY_BASIC_INFORMATION> {
    let mut regions = Vec::new();
    let mut address: usize = 0;

    loop {
      let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
      let result = unsafe {
        VirtualQueryEx(
          self.handle,
          Some(address as *mut _),
          &mut mbi,
          std::mem::size_of::<MEMORY_BASIC_INFORMATION>()
        )
      };

      if result == 0 {
        break; // No more regions
      }

      if (mbi.State & MEM_COMMIT) != VIRTUAL_ALLOCATION_TYPE(0) && mbi.Type == MEM_IMAGE {
        if let Ok(mapped_path) = Self::get_mapped_image_native_path(self.handle, mbi.BaseAddress as *mut u8) {
          if mapped_path == path {
            regions.push(mbi);
          }
        }
      }

      // Move to the next region
      address = mbi.BaseAddress as usize + mbi.RegionSize;
    }

    regions
  }

  pub fn get_committed_image_allocation(&self) -> Vec<MEMORY_BASIC_INFORMATION> {
    self.get_committed_image_allocation_with_path(&self.path_of().unwrap())
  }

  pub fn virtual_alloc(&self, size: usize) -> *mut u8 {
    unsafe {
      VirtualAllocEx(
        self.handle,
        None,
        size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE
      ) as *mut u8
    }
  }

  pub fn write_memory(&self, dst: *mut u8, data: &[u8]) {
    let mut written = 0;
    unsafe {
      WriteProcessMemory(
        self.handle,
        dst as _,
        data.as_ptr() as _,
        data.len(),
        Some(&mut written)
      )
      .unwrap();
    }
  }

  pub fn read_memory(&self, src: *const u8, dst: &mut [u8]) -> usize {
    let mut read = 0;
    unsafe {
      ReadProcessMemory(self.handle, src as _, dst.as_mut_ptr() as _, dst.len(), Some(&mut read)).unwrap();
    }
    read
  }
}
