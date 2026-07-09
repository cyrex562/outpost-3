//! Filesystem path utilities for Harsh Realm.
//!
//! Ported from `src/harsh_realm/paths.py`. Python resolved the base dir from
//! `__file__` (dev) or the PyInstaller `_MEIPASS` temp dir (frozen). Rust has no
//! source path at runtime, so the base dir comes from the `HARSH_REALM_BASE_DIR`
//! environment variable when set (the bundle/launcher sets it), otherwise the
//! current working directory.

use std::path::PathBuf;

/// Return the base directory of the application.
pub fn get_base_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HARSH_REALM_BASE_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Return the path to the compiled frontend static files.
pub fn get_frontend_dist_dir() -> PathBuf {
    get_base_dir().join("frontend").join("dist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_dist_is_under_base() {
        let base = get_base_dir();
        let dist = get_frontend_dist_dir();
        assert!(dist.starts_with(&base));
        assert!(dist.ends_with("frontend/dist"));
    }
}
