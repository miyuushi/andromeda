use std::os::windows::ffi::OsStringExt;
use std::{
  ffi::{CString, OsStr, OsString},
  os::windows::ffi::OsStrExt,
  path::{Path, PathBuf}
};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::{
  Foundation::{FreeLibrary, HMODULE},
  System::LibraryLoader::{GetModuleFileNameW, GetProcAddress, LoadLibraryW}
};
use windows::core::{HSTRING, PCSTR, PCWSTR};

pub struct Closeable<T: Default + Copy> {
  object: T,
  owned: bool,
  closer: fn(T)
}

impl<T: Default + Copy> Closeable<T> {
  pub fn new(object: T, owned: bool, closer: fn(T)) -> Self {
    Self { object, owned, closer }
  }

  pub fn attach(&mut self, object: T, owned: bool) {
    self.clear();
    self.object = object;
    self.owned = owned;
  }

  pub fn detach(&mut self) -> T
  where
    T: Default
  {
    self.owned = false;
    std::mem::replace(&mut self.object, Default::default())
  }

  pub fn clear(&mut self) {
    if self.owned {
      (self.closer)(self.object);
    }
    self.owned = false;
  }

  pub fn value(&self) -> &T {
    &self.object
  }

  pub fn value_mut(&mut self) -> &mut T {
    &mut self.object
  }

  pub fn has_ownership(&self) -> bool {
    self.owned
  }
}

impl<T: Default + Copy> Drop for Closeable<T> {
  fn drop(&mut self) {
    self.clear();
  }
}

impl<T: Default + Copy> Default for Closeable<T> {
  fn default() -> Self {
    Self {
      object: T::default(),
      owned: false,
      closer: |T| {}
    }
  }
}

#[derive(Default)]
pub struct LoadedModule {
  pub handle: Closeable<HMODULE>,
  pinned: bool
}

impl LoadedModule {
  /// Load a module by path
  pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
    let wide: Vec<u16> = OsStr::new(path.as_ref())
      .encode_wide()
      .chain(std::iter::once(0))
      .collect();

    let handle = unsafe { LoadLibraryW(PCWSTR(HSTRING::from_wide(&wide).as_ptr())) };
    if handle.is_err() {
      return Err(format!("Failed to load module {:?}", path.as_ref()));
    }

    Ok(Self {
      handle: Closeable::new(handle.unwrap(), true, |h| unsafe {
        FreeLibrary(h);
      }),
      pinned: false
    })
  }

  /// Get a function pointer from the module
  pub unsafe fn get_proc_address<T: Sized>(&self, name: &str) -> Option<T> {
    let name = CString::new(name).expect("CString had internal null byte present");
    let ptr = unsafe { GetProcAddress(*self.handle.value(), PCSTR(name.as_ptr() as *const u8)) };
    if ptr.is_none() {
      None
    } else {
      Some(unsafe { std::mem::transmute_copy::<*mut std::ffi::c_void, T>(&(ptr.unwrap() as *mut _)) })
    }
  }

  /// Full path of the module
  pub fn path_of(&self) -> PathBuf {
    let mut buffer = vec![0u16; MAX_PATH as usize]; // MAX_PATH
    let len = unsafe { GetModuleFileNameW(Some(*self.handle.value()), &mut buffer) };
    buffer.truncate(len as usize);
    PathBuf::from(OsString::from_wide(&buffer))
  }

  /// Just the file name
  pub fn base_name(&self) -> Option<PathBuf> {
    self.path_of().file_name().map(PathBuf::from)
  }
}

unsafe impl Send for LoadedModule {}
unsafe impl Sync for LoadedModule {}
