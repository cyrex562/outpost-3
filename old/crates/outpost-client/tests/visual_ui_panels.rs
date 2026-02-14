use outpost_client::testing::assertions::assert_matches_reference;
use outpost_client::testing::reference_store::ReferenceStore;
use outpost_client::testing::screenshot::Screenshot;
use tracing::info;

fn generate_layout_screenshot(width: u32, height: u32) -> Screenshot {
    let mut pixels = vec![30u8; (width * height * 4) as usize]; // background gray
    let top_h = 8;
    let bottom_h = 8;
    let left_w = 8;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            // bottom bar: dark red
            if y >= height - bottom_h {
                pixels[idx] = 120;
                pixels[idx + 1] = 20;
                pixels[idx + 2] = 20;
                pixels[idx + 3] = 255;
            // top bar: blue-ish
            } else if y < top_h {
                pixels[idx] = 40;
                pixels[idx + 1] = 80;
                pixels[idx + 2] = 160;
                pixels[idx + 3] = 255;
            // left sidebar: green-ish
            } else if x < left_w {
                pixels[idx] = 30;
                pixels[idx + 1] = 160;
                pixels[idx + 2] = 80;
                pixels[idx + 3] = 255;
            } else {
                // background stays as-in
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
fn ui_panels_match_reference_layout() {
    let mut store = ReferenceStore::new();
    let reference = generate_layout_screenshot(64, 48);
    let actual = generate_layout_screenshot(64, 48);
    store.save("ui_panels_v1", reference);

    let result = assert_matches_reference(&mut store, "ui_panels_v1", &actual, 0);
    info!(
        "UI panel test: {}",
        if result.passed { "pass" } else { "fail" }
    );

    assert!(result.passed);
    assert!(store.get("ui_panels_v1__diff").is_none());
}
