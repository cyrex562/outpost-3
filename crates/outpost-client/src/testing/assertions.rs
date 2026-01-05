use tracing::info;

use super::image_diff::{pixel_diff, ImageDiffResult};
use super::reference_store::ReferenceStore;
use super::screenshot::Screenshot;

#[derive(Debug, Clone, PartialEq)]
pub struct VisualAssertionResult {
    pub passed: bool,
    pub diff: Option<ImageDiffResult>,
}

impl VisualAssertionResult {
    pub fn passed() -> Self {
        Self {
            passed: true,
            diff: None,
        }
    }

    pub fn failed(diff: ImageDiffResult) -> Self {
        Self {
            passed: false,
            diff: Some(diff),
        }
    }
}

/// Assert that an actual screenshot matches a stored reference image within tolerance.
/// On failure, a diff image is saved back into the reference store under `{name}__diff`.
/// Logs "Visual assertion: {name} pass|fail".
pub fn assert_matches_reference(
    store: &mut ReferenceStore,
    name: &str,
    actual: &Screenshot,
    tolerance: u8,
) -> VisualAssertionResult {
    let Some(reference) = store.get(name).cloned() else {
        info!("Visual assertion: {} fail (missing reference)", name);
        return VisualAssertionResult::failed(ImageDiffResult {
            diff_pct: 100.0,
            diff_pixels: actual.width.saturating_mul(actual.height) as usize,
            total_pixels: actual.width.saturating_mul(actual.height) as usize,
        });
    };

    if reference.width != actual.width || reference.height != actual.height {
        info!("Visual assertion: {} fail (size mismatch)", name);
        return VisualAssertionResult::failed(ImageDiffResult {
            diff_pct: 100.0,
            diff_pixels: reference.width.saturating_mul(reference.height) as usize,
            total_pixels: reference.width.saturating_mul(reference.height) as usize,
        });
    }

    let diff = pixel_diff(&reference.pixels, &actual.pixels, tolerance);

    if diff.diff_pixels == 0 {
        info!("Visual assertion: {} pass", name);
        VisualAssertionResult::passed()
    } else {
        // Build a simple diff image: red for differing pixels, else actual pixel.
        let mut diff_pixels = Vec::with_capacity(actual.pixels.len());
        for (r_px, a_px) in reference
            .pixels
            .chunks_exact(4)
            .zip(actual.pixels.chunks_exact(4))
        {
            let different = r_px
                .iter()
                .zip(a_px.iter())
                .any(|(r, a)| r.abs_diff(*a) > tolerance);
            if different {
                diff_pixels.extend_from_slice(&[255, 0, 0, 255]);
            } else {
                diff_pixels.extend_from_slice(a_px);
            }
        }
        let diff_image = Screenshot {
            width: actual.width,
            height: actual.height,
            pixels: diff_pixels,
        };
        store.save(format!("{}__diff", name), diff_image);
        info!(
            "Visual assertion: {} fail ({:.2}% different)",
            name, diff.diff_pct
        );
        VisualAssertionResult::failed(diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_shot(pixels: &[u8], width: u32, height: u32) -> Screenshot {
        Screenshot {
            width,
            height,
            pixels: pixels.to_vec(),
        }
    }

    #[test]
    fn passes_when_identical() {
        let mut store = ReferenceStore::new();
        let shot = mk_shot(&[1, 2, 3, 4], 1, 1);
        store.save("button", shot.clone());

        let result = assert_matches_reference(&mut store, "button", &shot, 0);
        assert!(result.passed);
        assert!(result.diff.is_none());
        assert!(store.get("button__diff").is_none());
    }

    #[test]
    fn fails_and_saves_diff_when_different() {
        let mut store = ReferenceStore::new();
        let reference = mk_shot(&[0, 0, 0, 255], 1, 1);
        let actual = mk_shot(&[255, 255, 255, 255], 1, 1);
        store.save("panel", reference);

        let result = assert_matches_reference(&mut store, "panel", &actual, 0);
        assert!(!result.passed);
        assert!(result.diff.is_some());
        assert_eq!(result.diff.unwrap().diff_pixels, 1);
        let diff_image = store.get("panel__diff").unwrap();
        assert_eq!(diff_image.pixels, vec![255, 0, 0, 255]);
    }
}
