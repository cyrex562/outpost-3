//! Compiler diagnostics — the two author-facing buckets (grammar spec §7).
//!
//! `SyntaxError` = the author's text is malformed or an internal reference is
//! broken; they must fix it. `UnimplementedPrimitive` = a well-formed reference
//! to an engine primitive (verb/event/resource/…) that doesn't exist yet — the
//! author decides whether it's prototyping-ahead or a typo. The compiler collects
//! every diagnostic and fails atomically if either bucket is non-empty.

use serde::{Deserialize, Serialize};

/// Which bucket a diagnostic falls in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticClass {
    SyntaxError,
    UnimplementedPrimitive,
}

/// One diagnostic with an optional `file:line`-style location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub class: DiagnosticClass,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

impl Diagnostic {
    /// A malformed-text / broken-internal-reference diagnostic.
    pub fn syntax(message: impl Into<String>) -> Self {
        Self {
            class: DiagnosticClass::SyntaxError,
            message: message.into(),
            location: None,
        }
    }

    /// A well-formed reference to a missing engine primitive.
    pub fn unimplemented(message: impl Into<String>) -> Self {
        Self {
            class: DiagnosticClass::UnimplementedPrimitive,
            message: message.into(),
            location: None,
        }
    }

    /// Attach a location (the host supplies `file:line`).
    pub fn at(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn is_syntax(&self) -> bool {
        self.class == DiagnosticClass::SyntaxError
    }
}
