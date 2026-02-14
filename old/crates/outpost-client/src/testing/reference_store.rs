use std::collections::HashMap;

use tracing::info;

use super::screenshot::Screenshot;

/// Simple in-memory reference image store.
#[derive(Default)]
pub struct ReferenceStore {
    images: HashMap<String, Screenshot>,
}

impl ReferenceStore {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    /// Save or replace a reference image.
    pub fn save(&mut self, name: impl Into<String>, image: Screenshot) {
        self.images.insert(name.into(), image);
    }

    /// Retrieve a reference image by name.
    pub fn get(&self, name: &str) -> Option<&Screenshot> {
        let result = self.images.get(name);
        if result.is_some() {
            info!("Reference image loaded: {}", name);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_get_roundtrip() {
        let mut store = ReferenceStore::new();
        let img = Screenshot {
            width: 2,
            height: 1,
            pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        store.save("sample", img.clone());

        let fetched = store.get("sample").cloned();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap(), img);
    }

    #[test]
    fn missing_reference_returns_none() {
        let store = ReferenceStore::new();
        assert!(store.get("does_not_exist").is_none());
    }
}
