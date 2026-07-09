//! YAML-editor API model. Ported from `models/editor_api/yaml.py`.

use serde::{Deserialize, Serialize};

/// Request body for inline YAML file saves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlContentUpdateRequest {
    /// Raw YAML content.
    pub content: String,
}
