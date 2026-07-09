//! Tauri IPC commands backed by the native Rust core ([`harsh_core`]).
//!
//! These are the "core components in Rust" entry points. Each command is a thin
//! adapter over pure logic in the `harsh-core` crate, so the game logic stays
//! testable without a GUI and can be shared with non-Tauri tooling. As more of
//! the Python engine is ported, add the corresponding commands here and register
//! them in [`crate::run`].

use harsh_core::dice::{DiceError, DiceResult, DiceRoller};

// harsh-core's DiceRoller is generic over rand::Rng; we drive it with the
// thread-local RNG for production rolls.

/// Roll `num_dice` d`sides` plus `modifier`, in native Rust.
///
/// Mirrors `DiceRoller.roll`. Returns a structured error string on invalid input
/// so the frontend can surface it.
#[tauri::command]
pub fn roll_dice(num_dice: u32, sides: u32, modifier: i32) -> Result<DiceResult, String> {
    let mut roller = DiceRoller::new(rand::thread_rng());
    roller
        .roll(num_dice, sides, modifier)
        .map_err(|e: DiceError| e.to_string())
}

/// Roll a full 6-attribute set using 4d6-drop-lowest, in native Rust.
#[tauri::command]
pub fn roll_attribute_set() -> Vec<i32> {
    let mut roller = DiceRoller::new(rand::thread_rng());
    roller.roll_attribute_set()
}

/// Return the XWN attribute modifier for a score, in native Rust.
#[tauri::command]
pub fn attribute_modifier(score: i32) -> i32 {
    harsh_core::dice::attr_modifier(score)
}
