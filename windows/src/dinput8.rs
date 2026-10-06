//! Proxying `DirectInput8Create`, so that a tool shipped as `dinput8.dll` can
//! be loaded by the game at startup.

use std::ffi::c_void;
use std::{mem, ptr};

use windows::core::{s, w, GUID, HRESULT, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, MAX_PATH};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;

pub type FDirectInput8Create = unsafe extern "system" fn(
    hinst: HINSTANCE,
    dwversion: u32,
    riidltf: *const GUID,
    ppvout: *mut *mut c_void,
    punkouter: HINSTANCE,
) -> HRESULT;

/// Loads `DirectInput8Create` from the system's `dinput8.dll`.
///
/// `#[no_mangle]` exports must be defined in the tool's own cdylib, so each
/// tool keeps a `DirectInput8Create` forwarding to the function returned here:
///
/// ```ignore
/// static DIRECTINPUT8CREATE: Lazy<FDirectInput8Create> = Lazy::new(system_direct_input8_create);
///
/// #[no_mangle]
/// unsafe extern "system" fn DirectInput8Create(
///     hinst: HINSTANCE,
///     dwversion: u32,
///     riidltf: *const GUID,
///     ppvout: *mut *mut c_void,
///     punkouter: HINSTANCE,
/// ) -> HRESULT {
///     (DIRECTINPUT8CREATE)(hinst, dwversion, riidltf, ppvout, punkouter)
/// }
/// ```
pub fn system_direct_input8_create() -> FDirectInput8Create {
    unsafe {
        let mut dinput8_path = [0u16; MAX_PATH as usize];
        let count = GetSystemDirectoryW(Some(&mut dinput8_path)) as usize;

        // If count == 0, this will be fun
        ptr::copy_nonoverlapping(w!("\\dinput8.dll").0, dinput8_path[count..].as_mut_ptr(), 12);

        let dinput8 = LoadLibraryW(PCWSTR(dinput8_path.as_ptr())).unwrap();
        mem::transmute::<Option<unsafe extern "system" fn() -> isize>, FDirectInput8Create>(
            GetProcAddress(dinput8, s!("DirectInput8Create")),
        )
    }
}
