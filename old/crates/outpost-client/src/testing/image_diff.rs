use tracing::info;

/// Result of a pixel-by-pixel diff.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageDiffResult {
    pub diff_pct: f32,
    pub diff_pixels: usize,
    pub total_pixels: usize,
}

impl ImageDiffResult {
    pub fn is_identical(&self) -> bool {
        self.diff_pixels == 0
    }
}

/// Compute pixel difference percentage between two RGBA buffers (same length).
/// Pixels are considered different if any channel differs by more than `tolerance`.
/// Returns percentage in range [0, 100].
pub fn pixel_diff(a: &[u8], b: &[u8], tolerance: u8) -> ImageDiffResult {
    assert_eq!(a.len(), b.len(), "Buffers must be the same length");
    assert!(
        a.len() % 4 == 0,
        "Buffers must be RGBA (len divisible by 4)"
    );

    let mut diff_pixels = 0usize;
    let total_pixels = a.len() / 4;
    for (px_a, px_b) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        if px_a
            .iter()
            .zip(px_b.iter())
            .any(|(a, b)| a.abs_diff(*b) > tolerance)
        {
            diff_pixels += 1;
        }
    }

    let diff_pct = if total_pixels == 0 {
        0.0
    } else {
        (diff_pixels as f32) / (total_pixels as f32) * 100.0
    };

    info!("Image diff: {:.2}% different", diff_pct);

    ImageDiffResult {
        diff_pct,
        diff_pixels,
        total_pixels,
    }
}

/// Compute Structural Similarity Index (SSIM) for two RGBA buffers.
/// Converts to a simple luminance channel (RGB averaged, alpha ignored) and applies
/// the classic SSIM formula with constants C1=0.01^2, C2=0.03^2.
/// Returns a score in [0.0, 1.0], where 1.0 is identical.
pub fn ssim_rgba(a: &[u8], b: &[u8]) -> f32 {
    assert_eq!(a.len(), b.len(), "Buffers must be the same length");
    assert!(
        a.len() % 4 == 0,
        "Buffers must be RGBA (len divisible by 4)"
    );

    let total_pixels = a.len() / 4;
    if total_pixels == 0 {
        return 1.0;
    }

    // Convert to simple luminance in [0,1]
    let mut luminance_a = Vec::with_capacity(total_pixels);
    let mut luminance_b = Vec::with_capacity(total_pixels);
    for (px_a, px_b) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let la = (px_a[0] as f32 + px_a[1] as f32 + px_a[2] as f32) / (3.0 * 255.0);
        let lb = (px_b[0] as f32 + px_b[1] as f32 + px_b[2] as f32) / (3.0 * 255.0);
        luminance_a.push(la);
        luminance_b.push(lb);
    }

    let mean = |xs: &[f32]| xs.iter().sum::<f32>() / xs.len() as f32;
    let mu_x = mean(&luminance_a);
    let mu_y = mean(&luminance_b);

    let variance = |xs: &[f32], mu: f32| -> f32 {
        if xs.is_empty() {
            return 0.0;
        }
        xs.iter()
            .map(|x| {
                let d = x - mu;
                d * d
            })
            .sum::<f32>()
            / xs.len() as f32
    };

    let sigma_x2 = variance(&luminance_a, mu_x);
    let sigma_y2 = variance(&luminance_b, mu_y);

    let covariance = {
        if luminance_a.is_empty() {
            0.0
        } else {
            luminance_a
                .iter()
                .zip(luminance_b.iter())
                .map(|(x, y)| (x - mu_x) * (y - mu_y))
                .sum::<f32>()
                / luminance_a.len() as f32
        }
    };

    let c1 = 0.01f32 * 0.01f32;
    let c2 = 0.03f32 * 0.03f32;

    let numerator = (2.0 * mu_x * mu_y + c1) * (2.0 * covariance + c2);
    let denominator = (mu_x * mu_x + mu_y * mu_y + c1) * (sigma_x2 + sigma_y2 + c2);

    // Guard against numerical issues
    let raw = if denominator.abs() < f32::EPSILON {
        1.0
    } else {
        numerator / denominator
    };

    // Clamp to [0,1]
    let score = raw.clamp(0.0, 1.0);
    info!("SSIM: {:.4}", score);
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_buffers_have_zero_diff() {
        let a = vec![10u8, 20, 30, 255, 5, 5, 5, 255]; // 2 pixels
        let b = a.clone();
        let result = pixel_diff(&a, &b, 0);
        assert_eq!(result.diff_pixels, 0);
        assert_eq!(result.diff_pct, 0.0);
        assert!(result.is_identical());
    }

    #[test]
    fn diff_is_symmetric() {
        let a = vec![0u8, 0, 0, 255];
        let b = vec![255u8, 255, 255, 255];
        let r1 = pixel_diff(&a, &b, 0);
        let r2 = pixel_diff(&b, &a, 0);
        assert_eq!(r1.diff_pixels, r2.diff_pixels);
        assert!((r1.diff_pct - r2.diff_pct).abs() < f32::EPSILON);
    }

    #[test]
    fn tolerance_handles_antialiasing() {
        let a = vec![110u8, 110, 110, 255];
        let b = vec![120u8, 120, 120, 255];

        let strict = pixel_diff(&a, &b, 0);
        assert_eq!(strict.diff_pixels, 1);

        let tolerant = pixel_diff(&a, &b, 15);
        assert_eq!(tolerant.diff_pixels, 0);
        assert!(tolerant.is_identical());
    }

    #[test]
    #[should_panic]
    fn panics_on_mismatched_lengths() {
        let a = vec![0u8, 0, 0, 255];
        let b = vec![0u8, 0, 0];
        let _ = pixel_diff(&a, &b, 0);
    }

    #[test]
    fn ssim_identical_is_one() {
        let a = vec![100u8, 120, 140, 255, 50, 60, 70, 255];
        let b = a.clone();
        let score = ssim_rgba(&a, &b);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ssim_stays_within_bounds() {
        let a = vec![0u8, 0, 0, 255, 255, 255, 255, 255]; // black then white
        let b = vec![255u8, 255, 255, 255, 0, 0, 0, 255]; // inverted
        let score = ssim_rgba(&a, &b);
        assert!(score >= 0.0 && score <= 1.0);
    }
}
