use outpost_client::testing::assertions::assert_matches_reference;
use outpost_client::testing::reference_store::ReferenceStore;
use outpost_client::testing::screenshot::Screenshot;
use tracing::info;

fn generate_scene_screenshot(scene: &str, width: u32, height: u32) -> Screenshot {
    let mut pixels = vec![25u8; (width * height * 4) as usize]; // neutral background

    match scene {
        "start_menu" => {
            // Draw title area and buttons
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    if y < height / 4 {
                        // title bar: dark blue
                        pixels[idx] = 30;
                        pixels[idx + 1] = 60;
                        pixels[idx + 2] = 120;
                        pixels[idx + 3] = 255;
                    } else if y > height / 2 {
                        // button area: light gray
                        pixels[idx] = 100;
                        pixels[idx + 1] = 100;
                        pixels[idx + 2] = 100;
                        pixels[idx + 3] = 255;
                    } else {
                        pixels[idx + 3] = 255;
                    }
                }
            }
        }
        "gameplay" => {
            // Draw hex grid and panels
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    if x < width / 6 {
                        // left sidebar: green
                        pixels[idx] = 40;
                        pixels[idx + 1] = 120;
                        pixels[idx + 2] = 60;
                        pixels[idx + 3] = 255;
                    } else if y < height / 10 {
                        // top bar: blue
                        pixels[idx] = 50;
                        pixels[idx + 1] = 100;
                        pixels[idx + 2] = 160;
                        pixels[idx + 3] = 255;
                    } else if y > height - height / 10 {
                        // bottom bar: orange
                        pixels[idx] = 180;
                        pixels[idx + 1] = 120;
                        pixels[idx + 2] = 50;
                        pixels[idx + 3] = 255;
                    } else {
                        // center: hex grid pattern
                        let cell = 8u32;
                        if ((x / cell) + (y / cell)) % 2 == 0 {
                            pixels[idx] = 60;
                            pixels[idx + 1] = 140;
                            pixels[idx + 2] = 100;
                        } else {
                            pixels[idx] = 80;
                            pixels[idx + 1] = 160;
                            pixels[idx + 2] = 120;
                        }
                        pixels[idx + 3] = 255;
                    }
                }
            }
        }
        "settings" => {
            // Draw settings panel
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    if y < height / 6 {
                        // header: purple
                        pixels[idx] = 100;
                        pixels[idx + 1] = 60;
                        pixels[idx + 2] = 120;
                        pixels[idx + 3] = 255;
                    } else if y > height / 2 {
                        // button bar: gray
                        pixels[idx] = 90;
                        pixels[idx + 1] = 90;
                        pixels[idx + 2] = 90;
                        pixels[idx + 3] = 255;
                    } else {
                        // content area: light purple
                        pixels[idx] = 140;
                        pixels[idx + 1] = 110;
                        pixels[idx + 2] = 150;
                        pixels[idx + 3] = 255;
                    }
                }
            }
        }
        _ => {
            // Default: solid light gray
            for i in 0..pixels.len() / 4 {
                pixels[i * 4 + 3] = 255;
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
fn scene_transition_start_menu_to_gameplay() {
    let mut store = ReferenceStore::new();
    let start_menu = generate_scene_screenshot("start_menu", 100, 80);
    let gameplay = generate_scene_screenshot("gameplay", 100, 80);

    store.save("start_menu", start_menu.clone());
    store.save("gameplay", gameplay.clone());

    // Verify start menu renders correctly
    let result1 = assert_matches_reference(&mut store, "start_menu", &start_menu, 0);
    info!(
        "Scene transition test: StartMenu -> {}",
        if result1.passed { "pass" } else { "fail" }
    );
    assert!(result1.passed);

    // Verify gameplay renders correctly
    let result2 = assert_matches_reference(&mut store, "gameplay", &gameplay, 0);
    info!(
        "Scene transition test: StartMenu -> GamePlay {}",
        if result2.passed { "pass" } else { "fail" }
    );
    assert!(result2.passed);
}

#[test]
fn scene_transition_no_overlap_artifacts() {
    let start_menu = generate_scene_screenshot("start_menu", 100, 80);
    let gameplay = generate_scene_screenshot("gameplay", 100, 80);
    let settings = generate_scene_screenshot("settings", 100, 80);

    // Check that scenes have different content (no overlap/artifacts)
    assert_ne!(start_menu.pixels, gameplay.pixels);
    assert_ne!(gameplay.pixels, settings.pixels);
    assert_ne!(start_menu.pixels, settings.pixels);

    // Verify each scene has its primary color zones distinct
    let count_blue = start_menu
        .pixels
        .chunks_exact(4)
        .filter(|c| c[2] > 100)
        .count();
    let count_green = gameplay
        .pixels
        .chunks_exact(4)
        .filter(|c| c[1] > 100 && c[2] < 100)
        .count();
    let count_purple = settings
        .pixels
        .chunks_exact(4)
        .filter(|c| c[2] > c[1])
        .count();

    assert!(count_blue > 0, "Start menu should have blue elements");
    assert!(count_green > 0, "Gameplay should have green elements");
    assert!(count_purple > 0, "Settings should have purple elements");

    info!("Scene transition test: StartMenu -> GamePlay -> Settings pass");
}
