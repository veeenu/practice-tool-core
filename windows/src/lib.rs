//! Windows-specific components.
#![cfg(windows)]

use std::fs;
use std::path::PathBuf;

use windows::{
    core::w,
    Win32::System::Registry::{
        RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, REG_VALUE_TYPE,
    },
};

unsafe fn get_steam_path_from_registry() -> Result<PathBuf, String> {
    let mut key_handle = HKEY::default();

    RegOpenKeyExW(
        HKEY_LOCAL_MACHINE,
        w!(r"SOFTWARE\WOW6432Node\Valve\Steam"),
        0,
        KEY_READ,
        &mut key_handle,
    )
    .ok()
    .or_else(|_| {
        RegOpenKeyExW(HKEY_LOCAL_MACHINE, w!(r"SOFTWARE\Valve\Steam"), 0, KEY_READ, &mut key_handle)
            .ok()
    })
    .map_err(|e| format!("Could not open key: {e:?}"))?;

    let mut size = 0u32;
    let mut type_id = REG_VALUE_TYPE(0);
    let _ = RegQueryValueExW(
        key_handle,
        w!("InstallPath"),
        None,
        Some(&mut type_id),
        None,
        Some(&mut size),
    );

    if type_id.0 != REG_SZ.0 {
        return Err(format!("Wrong registry key type: {type_id:?}"));
    }

    let mut buffer = vec![0u8; size as usize];
    RegQueryValueExW(
        key_handle,
        w!("InstallPath"),
        None,
        Some(&mut type_id),
        Some(buffer.as_mut_ptr()),
        Some(&mut size),
    )
    .ok()
    .map_err(|e| format!("Couldn't query registry key value: {e:?}"))?;

    let wide_str = std::slice::from_raw_parts(buffer.as_ptr() as *const u16, buffer.len() / 2 - 1);

    String::from_utf16(wide_str)
        .map(PathBuf::from)
        .map_err(|e| format!("Couldn't decode wide string: {e:?}"))
}

pub fn find_steam_library_folders() -> Result<Vec<PathBuf>, String> {
    let steam_install_path =
        unsafe { get_steam_path_from_registry() }?.join("steamapps").join("libraryfolders.vdf");

    let vdf_content = fs::read_to_string(&steam_install_path)
        .map_err(|e| format!("Couldn't load VDF file from {steam_install_path:?}: {e:?}"))?;

    Ok(vdf_content
        .lines()
        .filter_map(|l| {
            if l.contains(r#""path""#) {
                l.split_whitespace().skip(1).next().map(|s| PathBuf::from(&s[1..s.len() - 1]))
            } else {
                None
            }
        })
        .collect())
}
