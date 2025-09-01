use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::{addr_of, null, null_mut};
use windows::Win32::System::Diagnostics::Debug::{
  IMAGE_DATA_DIRECTORY, IMAGE_DIRECTORY_ENTRY_IMPORT, IMAGE_NT_HEADERS64, IMAGE_NT_OPTIONAL_HDR64_MAGIC
};
#[cfg(target_pointer_width = "64")]
use windows::Win32::System::WindowsProgramming::IMAGE_THUNK_DATA64;
use windows::Win32::{
  Foundation::{HANDLE, HMODULE},
  System::{
    LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryA},
    Memory::{PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, PAGE_READONLY, VirtualProtect},
    SystemServices::{IMAGE_DOS_HEADER, IMAGE_IMPORT_BY_NAME, IMAGE_IMPORT_DESCRIPTOR},
    Threading::{GetCurrentProcess, GetCurrentProcessId}
  }
};
use windows::core::PCWSTR;

#[cfg(target_pointer_width = "64")]
type Thunk = IMAGE_THUNK_DATA64;
#[cfg(target_pointer_width = "64")]
type NtHeaders = IMAGE_NT_HEADERS64;
#[cfg(target_pointer_width = "32")]
type Thunk = IMAGE_THUNK_DATA32;
#[cfg(target_pointer_width = "32")]
type NtHeaders = IMAGE_NT_HEADERS32;

#[derive(Debug)]
pub struct IatHook {
  module: HMODULE,
  dll_name_lower: String,
  func_name: String,
  // Pointer to the IAT slot we patched:
  slot_ptr: *mut *const c_void,
  // Original function pointer saved for trampoline:
  original: *const c_void
}

unsafe impl Send for IatHook {}
unsafe impl Sync for IatHook {}

#[derive(Debug, thiserror::Error)]
pub enum IatError {
  #[error("Module not found")]
  ModuleNotFound,
  #[error("Invalid module headers")]
  BadPeHeaders,
  #[error("Import directory missing")]
  NoImportDir,
  #[error("Target import not found")]
  ImportNotFound,
  #[error("VirtualProtect failed")]
  ProtectFailed
}

/// Case-insensitive ASCII compare for PE import DLL names.
fn eq_ascii_ci(a: &str, b: &str) -> bool {
  a.eq_ignore_ascii_case(b)
}

unsafe fn pe_headers(module: HMODULE) -> Result<(&'static IMAGE_DOS_HEADER, &'static NtHeaders), IatError> {
  if module.is_invalid() {
    return Err(IatError::ModuleNotFound);
  }
  let base = module.0 as usize;
  let dos = &*(base as *const IMAGE_DOS_HEADER);
  if dos.e_magic != 0x5A4D {
    // "MZ"
    return Err(IatError::BadPeHeaders);
  }
  let nt = &*(((base as isize) + dos.e_lfanew as isize) as *const NtHeaders);
  if nt.Signature != 0x00004550 {
    // "PE\0\0"
    return Err(IatError::BadPeHeaders);
  }
  Ok((dos, nt))
}

unsafe fn import_directory(module: HMODULE) -> Result<(*const IMAGE_IMPORT_DESCRIPTOR, usize), IatError> {
  let (_dos, nt_any) = pe_headers(module)?;

  // Read OptionalHeader.Magic to discriminate 32 vs 64
  let is_64 = {
    let opt_magic = *(addr_of!((*nt_any).OptionalHeader) as *const u16);
    opt_magic == IMAGE_NT_OPTIONAL_HDR64_MAGIC.0
  };

  let (import_rva, size) = if is_64 {
    let nt: &IMAGE_NT_HEADERS64 = &*(nt_any as *const _ as *const NtHeaders);
    let dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT.0 as usize];
    (dir.VirtualAddress, dir.Size as usize)
  } else {
    let nt = &*(nt_any as *const _ as *const NtHeaders);
    let dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT.0 as usize];
    (dir.VirtualAddress, dir.Size as usize)
  };

  if import_rva == 0 || size == 0 {
    return Err(IatError::NoImportDir);
  }

  let base = module.0 as usize;
  let imp = (base + import_rva as usize) as *const IMAGE_IMPORT_DESCRIPTOR;
  Ok((imp, size))
}

/// Walks the IAT of `module`, locates (dll_name, func_name), patches the slot to `new_fn`, returns original.
unsafe fn patch_iat(
  module: HMODULE,
  dll_name: &str,
  func_name: &str,
  new_fn: *const c_void
) -> Result<(*mut *const c_void, *const c_void), IatError> {
  let (mut imp, _size) = import_directory(module)?;
  let base = module.0 as usize;

  let target_dll_ci = dll_name.to_ascii_lowercase();

  while (*imp).Name != 0 {
    let name_ptr = (base + (*imp).Name as usize) as *const u8;
    // Read C string
    let mut len = 0usize;
    while *name_ptr.add(len) != 0 {
      len += 1;
    }
    let dll = std::str::from_utf8(std::slice::from_raw_parts(name_ptr, len))
      .unwrap_or_default()
      .to_ascii_lowercase();

    if eq_ascii_ci(&dll, &target_dll_ci) {
      // Use OriginalFirstThunk to read names; FirstThunk is the IAT to patch
      let orig_thunk = (base + (*imp).Anonymous.OriginalFirstThunk as usize) as *const Thunk;
      let first_thunk = (base + (*imp).FirstThunk as usize) as *mut Thunk;

      let mut i = 0usize;
      loop {
        let ot = *orig_thunk.add(i);
        let ft = first_thunk.add(i);
        if ot.u1.Ordinal == 0 {
          break;
        }

        // Imported by name?
        let is_by_ordinal = (ot.u1.Ordinal as usize & (1usize << (size_of::<usize>() * 8 - 1)) as usize) != 0;
        if !is_by_ordinal {
          let name_rva = ot.u1.AddressOfData as usize;
          let ibn = &*((base + name_rva) as *const IMAGE_IMPORT_BY_NAME);
          let name_ptr = addr_of!(ibn.Name) as *const u8;

          let mut len = 0usize;
          while *name_ptr.add(len) != 0 {
            len += 1;
          }
          let imp_name = std::str::from_utf8(std::slice::from_raw_parts(name_ptr, len)).unwrap_or_default();

          if imp_name == func_name {
            // Found! Patch the IAT slot (FirstThunk points to function pointer array).
            let slot_ptr = (&mut ((*ft).u1.Function as usize)) as *mut usize as *mut *const c_void;

            // Make page writable
            let mut old: PAGE_PROTECTION_FLAGS = PAGE_READONLY;
            if VirtualProtect(
              slot_ptr as *const c_void,
              size_of::<*const c_void>(),
              PAGE_EXECUTE_READWRITE,
              &mut old
            )
            .is_err()
            {
              return Err(IatError::ProtectFailed);
            }

            let original = *slot_ptr;
            // Write our detour
            core::ptr::write_volatile(slot_ptr, new_fn);

            // Restore protection
            let mut _tmp: PAGE_PROTECTION_FLAGS = PAGE_READONLY;
            VirtualProtect(slot_ptr as *const c_void, size_of::<*const c_void>(), old, &mut _tmp);

            return Ok((slot_ptr, original));
          }
        }
        i += 1;
      }
    }

    imp = imp.add(1);
  }

  Err(IatError::ImportNotFound)
}

impl IatHook {
  /// Install an IAT hook into `module` (HMODULE, e.g. current EXE) for `dll_name!func_name`.
  /// Returns a ready-to-use hook object that can call `original()` and `uninstall()`.
  pub unsafe fn import_hook(
    module: Option<HMODULE>,
    dll_name: &str,
    func_name: &str,
    detour: *const c_void
  ) -> Result<Self, IatError> {
    let module = module.unwrap_or(GetModuleHandleW(PCWSTR::null()).unwrap());
    let (slot, original) = patch_iat(module, dll_name, func_name, detour)?;
    Ok(Self {
      module,
      dll_name_lower: dll_name.to_ascii_lowercase(),
      func_name: func_name.to_string(),
      slot_ptr: slot,
      original
    })
  }

  pub fn original(&self) -> *const c_void {
    self.original
  }

  pub unsafe fn uninstall(&mut self) -> Result<(), IatError> {
    if self.slot_ptr.is_null() {
      return Ok(());
    }
    // temporarily RWX
    let mut old: PAGE_PROTECTION_FLAGS = PAGE_READONLY;
    if VirtualProtect(
      self.slot_ptr as *const c_void,
      size_of::<*const c_void>(),
      PAGE_EXECUTE_READWRITE,
      &mut old
    )
    .is_err()
    {
      return Err(IatError::ProtectFailed);
    }
    core::ptr::write_volatile(self.slot_ptr, self.original);
    let mut _tmp: PAGE_PROTECTION_FLAGS = PAGE_READONLY;
    VirtualProtect(
      self.slot_ptr as *const c_void,
      size_of::<*const c_void>(),
      old,
      &mut _tmp
    );
    Ok(())
  }
}
