//! UI layout API model. Ported from `models/api/ui.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Saved UI layout document returned by `/api/ui/layout`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UILayoutState(pub JsonObject);
