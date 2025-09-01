mod d3d11;
mod dinput8;
mod dxgi;

use crate::utils::win32::dll::RealDll;

pub(crate) static DINPUT8: RealDll = RealDll::new("dinput8.dll");
pub(crate) static D3D11: RealDll = RealDll::new("d3d11.dll");
