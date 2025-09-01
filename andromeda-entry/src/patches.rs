use std::{
  ffi::{CStr, OsStr, c_uchar},
  iter::Once,
  os::windows::ffi::OsStrExt,
  ptr
};

use log::{error, info, warn};
use once_cell::sync::OnceCell;
use windows::{
  Win32::{
    Foundation::{ERROR_NOT_SUPPORTED, HANDLE, HMODULE, SetLastError},
    System::{
      Diagnostics::Debug::{
        IMAGE_DEBUG_DIRECTORY, IMAGE_DEBUG_TYPE_CODEVIEW, IMAGE_DIRECTORY_ENTRY_DEBUG, SYMBOL_INFO
      },
      LibraryLoader::{GetModuleHandleA, GetProcAddress},
      Memory::PAGE_READWRITE,
      SystemInformation::GetWindowsDirectoryW
    },
    UI::Shell::{PathIsRelativeW, PathIsSameRootW}
  },
  core::{BOOL, GUID, PCSTR, PCWSTR}
};

use crate::util::{LoadedModule, MemoryTenderizer};

static ORIG_SYM_FROM_ADDR: OnceCell<SymFromAddrFn> = OnceCell::new();
static ORIG_OPENPROCESS: OnceCell<OpenProcessFn> = OnceCell::new();

type SymFromAddrFn =
  unsafe extern "system" fn(hProcess: HANDLE, Address: u64, Displacement: *mut u64, Symbol: *mut SYMBOL_INFO) -> BOOL;

type OpenProcessFn = unsafe extern "system" fn(dwDesiredAccess: u32, bInheritHandle: BOOL, dwProcessId: u32) -> HANDLE;

pub(super) mod xiv {
  use once_cell::sync::OnceCell;
  use windows::{
    Win32::Foundation::{DUPLICATE_HANDLE_OPTIONS, ERROR_ACCESS_DENIED},
    core::BOOL
  };

  use crate::{
    DuplicateHandle, GetModuleHandleA, GetProcAddress, HANDLE, PCSTR, error, info,
    patches::{ORIG_OPENPROCESS, OpenProcessFn},
    utils::win32::iat::IatHook
  };

  static IAT_OPENPROCESS: OnceCell<IatHook> = OnceCell::new();

  unsafe extern "system" fn open_process_hook(dwDesiredAccess: u32, bInheritHandle: BOOL, dwProcessId: u32) -> HANDLE {
    info!("[HOOK] IAT OpenProcess called (process: {:?})", dwProcessId);
    let iat = IAT_OPENPROCESS.get().expect("IAT for OpenProcess not found");
    let orig: OpenProcessFn = std::mem::transmute(iat.original());
    let self_pid = windows::Win32::System::Threading::GetCurrentProcessId();
    if dwProcessId == self_pid && dwDesiredAccess & 0x20 != 0 {
      // PROCESS_VM_WRITE
      windows::Win32::Foundation::SetLastError(ERROR_ACCESS_DENIED);
      return HANDLE::default();
    }
    orig(dwDesiredAccess, bInheritHandle, dwProcessId)
  }

  unsafe extern "system" fn redirect_open_process_hook(
    dwDesiredAccess: u32,
    bInheritHandle: BOOL,
    dwProcessId: u32
  ) -> HANDLE {
    info!("[HOOK] Global OpenProcess called (process: {:?})", dwProcessId);
    let orig = ORIG_OPENPROCESS.get().expect("orig OpenProcess not found");
    let self_pid = windows::Win32::System::Threading::GetCurrentProcessId();
    if dwProcessId == self_pid {
      let current = windows::Win32::System::Threading::GetCurrentProcess();
      let mut dup = HANDLE(std::ptr::null_mut());
      if DuplicateHandle(
        current,
        current,
        current,
        &mut dup,
        dwDesiredAccess,
        bInheritHandle.as_bool(),
        DUPLICATE_HANDLE_OPTIONS(0)
      )
      .is_ok()
      {
        return dup;
      }
      return HANDLE::default();
    }
    orig(dwDesiredAccess, bInheritHandle, dwProcessId)
  }

  pub(super) fn disable_openprocess_access_check() {
    info!("[disable_openprocess_access_check]");

    let hook = unsafe { IatHook::import_hook(None, "kernel32.dll", "OpenProcess", open_process_hook as *mut _) };

    if let Err(err) = hook {
      error!("Failed to hook IAT for OpenProcess: {}", err);
      return;
    }

    if let Ok(iat) = hook {
      IAT_OPENPROCESS.get_or_init(|| iat);
    }
  }

  pub(super) fn redirect_openprocess() {
    info!("[redirect_openprocess]");

    let module = unsafe { GetModuleHandleA(PCSTR(c"kernel32.dll".as_ptr() as *mut u8)).unwrap() };
    let proc = unsafe { GetProcAddress(module, PCSTR(c"OpenProcess".as_ptr() as *mut u8)) };

    if proc.is_none() {
      error!("Failed to get OpenProcess address.");
      return;
    }

    unsafe {
      let _ = min_hook_rs::create_hook(proc.unwrap() as *mut _, redirect_open_process_hook as *mut _)
        .expect("Failed to hook OpenProcess");
      min_hook_rs::enable_hook(proc.unwrap() as *mut _);
    }
  }
}

unsafe extern "system" fn sym_from_addr_hook(
  hProcess: HANDLE,
  address: u64,
  displacement: *mut u64,
  symbol: *mut SYMBOL_INFO
) -> BOOL {
  info!(
    "[HOOK] SymFromAddr called for address: 0x{:X} (process: {:?})",
    address, hProcess
  );

  info!("Suppressed SymInitialize");
  unsafe { SetLastError(ERROR_NOT_SUPPORTED) };
  false.into()
}

fn symbol_load_patches() {
  info!("[symbol_load_patches]");

  let module = unsafe { GetModuleHandleA(PCSTR(c"dbghelp.dll".as_ptr() as *mut u8)).unwrap() };
  let proc = unsafe { GetProcAddress(module, PCSTR(c"SymFromAddr".as_ptr() as *mut u8)) };

  if proc.is_none() {
    error!("Failed to get SymFromAddr address.");
    return;
  }

  unsafe {
    let _ = min_hook_rs::create_hook(proc.unwrap() as *mut _, sym_from_addr_hook as *mut _)
      .expect("Failed to hook SymFromAddr");
    min_hook_rs::enable_hook(proc.unwrap() as *mut _);
  }
}

pub fn apply_all_patches() {
  let patches = vec![
    symbol_load_patches,
    xiv::redirect_openprocess,
    xiv::disable_openprocess_access_check,
  ];

  warn!("Applying all patches to running process..");

  for patch in patches {
    patch();
  }
}
