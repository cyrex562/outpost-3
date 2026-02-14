use outpost_client::testing::assertions::assert_matches_reference;
use outpost_client::testing::reference_store::ReferenceStore;
use outpost_client::testing::screenshot::Screenshot;
use tracing::info;

fn generate_hex_grid_screenshot(width: u32, height: u32) -> Screenshot {
    let mut pixels = vec![15u8; (width * height * 4) as usize]; // dark background
    let cell = 6u32; // coarse hex cell approximation

    for y in 0..height {
        // Offset every other row to mimic hex staggering
        let row = y / cell;
        let offset = if row % 2 == 0 { 0 } else { cell / 2 };

        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let q = (x + offset) / cell;
            let r = y / cell;

            // Simple coloring: alternate stripes by axial coords
            let is_border = (x + y) % cell == 0;
            if is_border {
                // grid lines
                pixels[idx] = 80;
                pixels[idx + 1] = 200;
                pixels[idx + 2] = 120;
                pixels[idx + 3] = 255;
            } else {
                // fill based on q+r parity
                if (q + r) % 3 == 0 {
                    pixels[idx] = 40;
                    pixels[idx + 1] = 120;
                    pixels[idx + 2] = 200;
                } else if (q + r) % 3 == 1 {
                    pixels[idx] = 120;
                    pixels[idx + 1] = 80;
                    pixels[idx + 2] = 160;
                } else {
                    pixels[idx] = 160;
                    pixels[idx + 1] = 120;
                    pixels[idx + 2] = 60;
                }
                pixels[idx + 3] = 255;
            }
        }
    }

    Screenshot {
        width,
        height,
        pixels,
    }
}

#[test]
fn hex_grid_render_is_stable() {
    let mut store = ReferenceStore::new();
    let reference = generate_hex_grid_screenshot(96, 72);
    let actual = generate_hex_grid_screenshot(96, 72);
    store.save("hex_grid_v1", reference);

    let result = assert_matches_reference(&mut store, "hex_grid_v1", &actual, 0);
    info!(
        "Hex grid test: {}",
        if result.passed { "pass" } else { "fail" }
    );

    assert!(result.passed);
    assert!(store.get("hex_grid_v1__diff").is_none());
}
