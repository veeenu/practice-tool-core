//! `XInputGetState` hook. It snapshots controller 0 into
//! [`GAMEPAD_STATE`], and hides controller input from the game while
//! [`BLOCK_XINPUT`] is set.

use std::ffi::{c_void, OsString};
use std::mem;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;

use hudhook::mh::{MH_ApplyQueued, MH_Initialize, MhHook, MH_STATUS};
use practice_tool_core::gamepad::{BLOCK_XINPUT, GAMEPAD_STATE};
use windows::core::{s, HSTRING};
use windows::Win32::Foundation::{ERROR_SUCCESS, MAX_PATH};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::UI::Input::XboxController::XINPUT_STATE;

type FXInputGetState =
    unsafe extern "system" fn(dw_user_index: u32, xinput_state: *mut XINPUT_STATE) -> u32;

struct Hook {
    trampoline: FXInputGetState,
    /// Adjusts the state handed to the game.
    filter: Option<fn(&mut XINPUT_STATE)>,
}

static HOOK: OnceLock<Hook> = OnceLock::new();

/// Hooks `XInputGetState` in `<system directory>\<dll_name>`. `filter`, if
/// any, adjusts the state handed to the game when it isn't blocked (e.g. to
/// apply a stick deadzone); [`GAMEPAD_STATE`] always gets the unmodified one.
///
/// Call this from `DllMain`, or early in the tool's startup, before the game
/// polls the controller. Tools then read [`GAMEPAD_STATE`] instead of calling
/// XInput. Once the hook is installed, further calls do nothing.
pub fn hook_xinput(dll_name: &str, filter: Option<fn(&mut XINPUT_STATE)>) -> Result<(), String> {
    if HOOK.get().is_some() {
        return Ok(());
    }

    unsafe {
        let mut path = [0u16; MAX_PATH as usize];
        let count = GetSystemDirectoryW(Some(&mut path)) as usize;
        if count == 0 || count >= path.len() {
            return Err("XInput hook: could not get system directory".to_string());
        }
        let path = PathBuf::from(OsString::from_wide(&path[..count])).join(dll_name);

        let lib = LoadLibraryW(&HSTRING::from(path.as_path()))
            .map_err(|e| format!("XInput hook: load {path:?}: {e}"))?;

        let xinput_get_state_addr = GetProcAddress(lib, s!("XInputGetState"))
            .ok_or_else(|| format!("XInput hook: XInputGetState not found in {path:?}"))?;

        match MH_Initialize() {
            MH_STATUS::MH_ERROR_ALREADY_INITIALIZED | MH_STATUS::MH_OK => {},
            status => return Err(format!("XInput hook: initialize: {status:?}")),
        }

        let hook =
            MhHook::new(xinput_get_state_addr as *mut c_void, xinput_get_state_impl as *mut c_void)
                .map_err(|status| format!("XInput hook: create: {status:?}"))?;

        // The detour reads the trampoline, so it must be set before the hook is
        // enabled. It can't be set already: creating a second hook for the same
        // function fails.
        let trampoline = mem::transmute::<*mut c_void, FXInputGetState>(hook.trampoline());
        let _ = HOOK.set(Hook { trampoline, filter });

        hook.queue_enable().map_err(|status| format!("XInput hook: queue enable: {status:?}"))?;
        MH_ApplyQueued().ok().map_err(|status| format!("XInput hook: apply queued: {status:?}"))?;
    }

    Ok(())
}

unsafe extern "system" fn xinput_get_state_impl(
    dw_user_index: u32,
    xinput_state: *mut XINPUT_STATE,
) -> u32 {
    let Hook { trampoline, filter } = HOOK.get().expect("XInput hook: trampoline not set");

    let r = trampoline(dw_user_index, xinput_state);

    // Save the unmodified state for the radial menu, before it is blocked or
    // filtered for the game.
    if dw_user_index == 0 {
        GAMEPAD_STATE.store(if r == ERROR_SUCCESS.0 { xinput_state.as_ref() } else { None });
    }

    if BLOCK_XINPUT.load(Ordering::SeqCst) {
        *xinput_state = Default::default();
        return r;
    }

    if r != ERROR_SUCCESS.0 {
        return r;
    }

    if let (Some(filter), Some(state)) = (filter, xinput_state.as_mut()) {
        filter(state);
    }

    r
}
