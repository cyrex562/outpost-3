//! Filesystem paths for built-in pack content and editor metadata.
//!
//! Ported from `src/harsh_realm/packs/paths.py` (post HR-413/HR-414 layout,
//! where packs live under `content/` and editor schemas under `content/schemas`).

use std::path::PathBuf;

use crate::paths::get_base_dir;

/// Return the built-in xwn-core content directory, falling back to legacy `data/`.
pub fn default_content_dir() -> PathBuf {
    let base = get_base_dir();

    // Priority 1: content/xwn-core/content
    let cwd_content = base.join("content").join("xwn-core").join("content");
    if cwd_content.exists() {
        return cwd_content;
    }

    // Priority 2: data/
    let cwd_data = base.join("data");
    if cwd_data.exists() {
        return cwd_data;
    }

    // Fallback for development if base_dir is weirdly set.
    PathBuf::from("content").join("xwn-core").join("content")
}

/// Return the engine-owned editor schema directory.
pub fn editor_schema_dir() -> PathBuf {
    let base = get_base_dir();
    let cwd_schema = base.join("content").join("schemas");
    if cwd_schema.exists() {
        return cwd_schema;
    }
    PathBuf::from("content").join("schemas")
}
