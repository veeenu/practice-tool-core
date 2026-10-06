//! Version of the game, read from its executable.

use std::ptr::null_mut;
use std::sync::OnceLock;

use hudhook::tracing::info;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK};

use crate::message_box;

/// File version of the process' executable, as `(major, minor, patch)`.
pub fn exe_file_version() -> (u32, u32, u32) {
    // Zero-filled, so it stays null-terminated.
    let mut file_path = vec![0u16; MAX_PATH as usize];
    unsafe { GetModuleFileNameW(Some(GetModuleHandleW(None).unwrap()), &mut file_path) };

    let mut version_info_size =
        unsafe { GetFileVersionInfoSizeW(PCWSTR(file_path.as_ptr()), None) };
    let mut version_info_buf = vec![0u8; version_info_size as usize];
    unsafe {
        GetFileVersionInfoW(
            PCWSTR(file_path.as_ptr()),
            None,
            version_info_size,
            version_info_buf.as_mut_ptr() as _,
        )
        .unwrap()
    };

    let mut version_info: *mut VS_FIXEDFILEINFO = null_mut();
    let _ = unsafe {
        VerQueryValueW(
            version_info_buf.as_ptr() as _,
            w!("\\\\\0"),
            &mut version_info as *mut *mut _ as _,
            &mut version_info_size,
        )
    };
    let version_info = unsafe { version_info.as_ref().unwrap() };
    let major = (version_info.dwFileVersionMS >> 16) & 0xffff;
    let minor = (version_info.dwFileVersionMS) & 0xffff;
    let patch = (version_info.dwFileVersionLS >> 16) & 0xffff;

    info!("Version {major} {minor} {patch}");
    (major, minor, patch)
}

/// Ensures that `cell` holds the game's version, or returns the unsupported
/// version after telling the user. The caller must exit cleanly on error.
pub fn check_version<V: TryFrom<(u32, u32, u32)> + Copy>(
    cell: &OnceLock<V>,
    tool_name: &str,
) -> Result<V, (u32, u32, u32)> {
    if let Some(version) = cell.get().copied() {
        return Ok(version);
    }

    let (major, minor, patch) = exe_file_version();
    match V::try_from((major, minor, patch)) {
        Ok(version) => Ok(*cell.get_or_init(|| version)),
        Err(_) => {
            message_box(
                &format!("{tool_name} - Unsupported version"),
                &format!(
                    "The current game version, {major}.{minor}.{patch}, is not supported \
                     yet.\n\nAn update will be released soon, please stay tuned!"
                ),
                MB_OK | MB_ICONERROR,
            );
            Err((major, minor, patch))
        },
    }
}
