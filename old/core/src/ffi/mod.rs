use crate::systems::ReducerContext;
use crate::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Opaque handle to GameState
#[repr(C)]
pub struct GameStateHandle {
    _private: [u8; 0],
}

/// Create new game state
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_state_new() -> *mut GameStateHandle {
    let state = Box::new(GameState::new());
    Box::into_raw(state) as *mut GameStateHandle
}

/// Free game state
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_state_free(handle: *mut GameStateHandle) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle as *mut GameState));
        }
    }
}

/// Apply command (returns JSON events)
/// Caller must free returned string with game_string_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_state_apply_command(
    handle: *mut GameStateHandle,
    command_json: *const c_char,
) -> *mut c_char {
    let state = unsafe { &mut *(handle as *mut GameState) };
    let json_str = unsafe { CStr::from_ptr(command_json) }.to_str().unwrap();

    let command: Command = serde_json::from_str(json_str).unwrap();
    let ctx = ReducerContext {
        current_offset: 0,
        game_time: state.game_time,
    };

    let (new_state, events) = systems::reduce(state.clone(), command, ctx);
    *state = new_state;

    let events_json = serde_json::to_string(&events).unwrap();
    CString::new(events_json).unwrap().into_raw()
}

/// Get state as JSON
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_state_to_json(handle: *const GameStateHandle) -> *mut c_char {
    let state = unsafe { &*(handle as *const GameState) };
    let json = serde_json::to_string(state).unwrap();
    CString::new(json).unwrap().into_raw()
}

/// Free string returned by Rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}
