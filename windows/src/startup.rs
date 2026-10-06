//! Starting the tool, either injected by its exe or loaded by the game at
//! startup as its `dinput8.dll`.
//!
//! Two named events coordinate the copies of the DLL and the exe: the running
//! event exists while a copy of the tool runs, and the start event lets the exe
//! start a copy that the game loaded at startup.

use std::ffi::c_void;
use std::time::{Duration, Instant};
use std::{env, thread};

use hudhook::tracing::error;
use windows::core::HSTRING;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CreateEventW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE, INFINITE,
    SYNCHRONIZATION_SYNCHRONIZE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RSHIFT};

/// Names of a tool's events.
pub struct Events {
    /// Created by the copy of the tool that starts: while it exists, the tool
    /// is running.
    running: HSTRING,
    /// Set by the exe to start the copy that the game loaded at startup.
    start: HSTRING,
}

impl Events {
    /// Events of the tool `tool_id`, e.g. `"jdsd_dsiii_practice_tool"`.
    pub fn new(tool_id: &str) -> Self {
        Events {
            running: format!("Local\\{tool_id}_running").into(),
            start: format!("Local\\{tool_id}_start").into(),
        }
    }
}

/// Starts the tool by calling `start` on a new thread. Call this from
/// `DllMain` on `DLL_PROCESS_ATTACH`.
///
/// `loaded_at_startup` is whether the game loaded the DLL at startup, i.e.
/// `DllMain`'s `reserved` argument isn't null. If it did, the tool waits until
/// right shift is held for 2 seconds within the first 10 seconds, or until the
/// exe sets the start event. Otherwise the exe injected it, and it starts right
/// away. Either way, it doesn't start if another copy is already running.
pub fn on_process_attach(
    events: Events,
    loaded_at_startup: bool,
    start: impl FnOnce() + Send + 'static,
) {
    // Injected by the exe: start right away. The running event must be claimed
    // before returning, as the exe checks for it as soon as the injection
    // completes.
    if !loaded_at_startup {
        if claim_running(&events) {
            thread::spawn(start);
        }
        return;
    }

    // Otherwise the game loaded it at startup as its `dinput8.dll`: wait to be
    // asked to start. The handle is never closed: the event must exist for as
    // long as the process.
    let start_event = match unsafe { CreateEventW(None, true, false, &events.start) } {
        Ok(start_event) => start_event.0 as usize,
        Err(e) => {
            error!("Couldn't create start event: {e:?}");
            return;
        },
    };

    thread::spawn(move || {
        if (env_start_requested() || await_start(HANDLE(start_event as *mut c_void)))
            && claim_running(&events)
        {
            start()
        }
    });
}

/// Whether a copy of the tool is running in the game.
pub fn is_running(events: &Events) -> bool {
    unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, false, &events.running) }.is_ok()
}

/// Starts the copy of the tool that the game loaded at startup.
pub fn request_start(events: &Events) -> windows::core::Result<()> {
    unsafe { OpenEventW(EVENT_MODIFY_STATE, false, &events.start).and_then(|e| SetEvent(e)) }
}

/// Waits until right shift is held for 2 seconds within the first 10 seconds,
/// or until the exe sets the start event. Returns `false` if waiting on the
/// event fails.
fn await_start(start_event: HANDLE) -> bool {
    let duration_threshold = Duration::from_secs(2);
    let check_window = Duration::from_secs(10);
    let poll_interval_ms = 100;

    let start_time = Instant::now();
    let mut key_down_start: Option<Instant> = None;

    while start_time.elapsed() < check_window {
        let state = unsafe { GetAsyncKeyState(VK_RSHIFT.0 as i32) };
        let key_down = state < 0;

        match (key_down, key_down_start) {
            (true, None) => {
                key_down_start = Some(Instant::now());
            },
            (true, Some(start)) => {
                if start.elapsed() >= duration_threshold {
                    return true;
                }
            },
            (false, _) => {
                key_down_start = None;
            },
        }

        if unsafe { WaitForSingleObject(start_event, poll_interval_ms) } == WAIT_OBJECT_0 {
            return true;
        }
    }

    unsafe { WaitForSingleObject(start_event, INFINITE) == WAIT_OBJECT_0 }
}

fn env_start_requested() -> bool {
    if env::var("DOLL_SKIP").ok().map(|s| s == "consistent").unwrap_or(false) {
        thread::sleep(Duration::from_millis(2000));
        true
    } else {
        false
    }
}

/// Marks the tool as running in this process. Returns `false` if another copy
/// already did.
fn claim_running(events: &Events) -> bool {
    // The handle is never closed: the event must exist for as long as the
    // process.
    match unsafe { CreateEventW(None, true, false, &events.running) } {
        Ok(_) => unsafe { GetLastError() != ERROR_ALREADY_EXISTS },
        Err(e) => {
            error!("Couldn't create running event: {e:?}");
            false
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_names() {
        let events = Events::new("x");
        assert_eq!(events.running, "Local\\x_running");
        assert_eq!(events.start, "Local\\x_start");
    }
}
