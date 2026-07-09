//! Pack subsystem exceptions. Ported from `models packs/exceptions.py`.

use std::fmt;

/// Errors raised by pack loading and resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// A pack directory or content file cannot be loaded.
    Load(String),
    /// A set of loaded packs cannot be resolved together.
    Resolution(String),
    /// A pack migration cannot be discovered or executed.
    Migration(String),
    /// Code-bearing pack registration fails.
    Code(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::Load(m) => write!(f, "{m}"),
            PackError::Resolution(m) => write!(f, "{m}"),
            PackError::Migration(m) => write!(f, "{m}"),
            PackError::Code(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for PackError {}
