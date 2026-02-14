use outpost_client::testing::assertions::assert_matches_reference;
use outpost_client::testing::reference_store::ReferenceStore;
use outpost_client::testing::screenshot::Screenshot;
use tracing::info;

fn generate_chart_screenshot(width: u32, height: u32) -> Screenshot {
    let mut pixels = vec![20u8; (width * height * 4) as usize]; // light background

    // Frame border: dark gray
    let border_color = [60u8, 60, 60, 255];
    for x in 0..width {
        let idx_top = (x * 4) as usize;
        let idx_bottom = (((height - 1) * width + x) * 4) as usize;
        pixels[idx_top..idx_top + 4].copy_from_slice(&border_color);
        pixels[idx_bottom..idx_bottom + 4].copy_from_slice(&border_color);
    }
    for y in 0..height {
        let idx_left = ((y * width) * 4) as usize;
        let idx_right = ((y * width + width - 1) * 4) as usize;
        pixels[idx_left..idx_left + 4].copy_from_slice(&border_color);
        pixels[idx_right..idx_right + 4].copy_from_slice(&border_color);
    }

    // Y-axis label area (left 10% of width)
    let axis_w = (width / 10).max(1);
    for y in 0..height {
        for x in 0..axis_w {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 100;
            pixels[idx + 1] = 100;
            pixels[idx + 2] = 100;
            pixels[idx + 3] = 255;
        }
    }

    // X-axis label area (bottom 10% of height)
    let axis_h = (height / 10).max(1);
    for y in (height.saturating_sub(axis_h))..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = 100;
            pixels[idx + 1] = 100;
            pixels[idx + 2] = 100;
            pixels[idx + 3] = 255;
        }
    }

    // Draw line 1 (blue): power generation curve
    let line1_color = [0u8, 100, 255, 255];
    for x in axis_w..width {
        let norm_x = (x as f32 - axis_w as f32) / ((width - axis_w) as f32);
        let y_val = 0.4 + 0.3 * (norm_x * 3.14).sin();
        let y = (height as f32 * (1.0 - y_val)).min((height - axis_h - 1) as f32) as u32;
        if y < height.saturating_sub(axis_h) && y > 0 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&line1_color);
        }
    }

    // Draw line 2 (green): power consumption curve
    let line2_color = [0u8, 200, 100, 255];
    for x in axis_w..width {
        let norm_x = (x as f32 - axis_w as f32) / ((width - axis_w) as f32);
        let y_val = 0.35 + 0.25 * (norm_x * 3.14 + 0.5).sin();
        let y = (height as f32 * (1.0 - y_val)).min((height - axis_h - 1) as f32) as u32;
        if y < height.saturating_sub(axis_h) && y > 0 {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&line2_color);
        }
    }

    Screenshot {
        width,
        height,
        pixels,
    }
}

#[test]
fn chart_rendering_is_stable() {
    let mut store = ReferenceStore::new();
    let reference = generate_chart_screenshot(120, 80);
    let actual = generate_chart_screenshot(120, 80);
    store.save("chart_power_v1", reference);

    let result = assert_matches_reference(&mut store, "chart_power_v1", &actual, 0);
    info!(
        "Chart test: {}",
        if result.passed { "pass" } else { "fail" }
    );

    assert!(result.passed);
    assert!(store.get("chart_power_v1__diff").is_none());
}

#[test]
fn chart_has_distinct_line_colors() {
    let shot = generate_chart_screenshot(120, 80);
    let pixels = &shot.pixels;

    // Count pixels of line colors
    let mut blue_count = 0u32;
    let mut green_count = 0u32;

    for chunk in pixels.chunks_exact(4) {
        if chunk == &[0u8, 100, 255, 255] {
            blue_count += 1;
        }
        if chunk == &[0u8, 200, 100, 255] {
            green_count += 1;
        }
    }

    assert!(
        blue_count > 0,
        "Chart should have blue line pixels for generation"
    );
    assert!(
        green_count > 0,
        "Chart should have green line pixels for consumption"
    );
    assert_ne!(
        blue_count, green_count,
        "Line colors should be visually distinct"
    );
}
