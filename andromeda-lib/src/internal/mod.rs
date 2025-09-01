pub(crate) mod interfaces;
pub(crate) mod swapchain_util;

use std::sync::{Mutex, OnceLock};

use crate::internal::interfaces::Interfaces;

pub(crate) static INTERFACES: OnceLock<Mutex<Interfaces>> = OnceLock::new();
