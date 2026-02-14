use bevy::render::texture::Image;
use tracing::info;

/// Represents a captured screenshot in RGBA8 format.
#[derive(Debug, Clone, PartialEq)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Screenshot {
    /// Counts the number of pixels that are not fully black (any channel > 0).
    pub fn non_black_pixel_count(&self) -> usize {
        self.pixels
            .chunks_exact(4)
            .filter(|px| px.iter().any(|&v| v != 0))
            .count()
    }
}

/// Capture a screenshot from a Bevy `Image` (desktop builds).
#[cfg(not(target_arch = "wasm32"))]
pub fn capture_from_image(image: &Image) -> Screenshot {
    let size = image.texture_descriptor.size;
    let width = size.width;
    let height = size.height;
    let pixels = image.data.clone();

    info!(
        "Screenshot captured: {}x{}, {} bytes",
        width,
        height,
        pixels.len()
    );

    Screenshot {
        width,
        height,
        pixels,
    }
}

/// Capture a screenshot from a Bevy `Image` (WASM builds).
#[cfg(target_arch = "wasm32")]
pub fn capture_from_image(image: &Image) -> Screenshot {
    let size = image.texture_descriptor.size;
    let width = size.width;
    let height = size.height;
    let pixels = image.data.clone();

    info!(
        "WASM screenshot captured: {}x{}, {} bytes",
        width,
        height,
        pixels.len()
    );

    Screenshot {
        width,
        height,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::render_asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    #[test]
    fn screenshot_dimensions_match_image() {
        let size = Extent3d {
            width: 4,
            height: 2,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[255, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );

        let shot = capture_from_image(&image);
        assert_eq!(shot.width, 4);
        assert_eq!(shot.height, 2);
        assert_eq!(shot.pixels.len(), (4 * 2 * 4) as usize);
    }

    #[test]
    fn screenshot_contains_non_black_pixels() {
        let size = Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[10, 20, 30, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );

        let shot = capture_from_image(&image);
        assert!(shot.non_black_pixel_count() >= (size.width * size.height) as usize);
        assert!(shot.pixels.iter().any(|v| *v != 0));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use bevy::render::render_asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    #[test]
    fn wasm_capture_preserves_dimensions() {
        let size = Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );

        let shot = capture_from_image(&image);
        assert_eq!(shot.width, 3);
        assert_eq!(shot.height, 1);
        assert_eq!(shot.pixels.len(), (3 * 1 * 4) as usize);
    }

    #[test]
    fn wasm_capture_uses_rgba8() {
        let size = Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[5, 6, 7, 8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );

        let shot = capture_from_image(&image);
        assert_eq!(shot.pixels, vec![5, 6, 7, 8]);
        assert_eq!(shot.non_black_pixel_count(), 1);
    }
}
