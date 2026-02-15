# Visual Testing System

## Overview

The Outpost-3 visual testing system provides deterministic, pixel-perfect validation of UI rendering across desktop and WASM builds. Unlike traditional screenshot-based testing that requires a running graphics backend, our system uses synthetic rendering to generate predictable pixel buffers that can be compared programmatically.

## Architecture

### Core Components

1. **Screenshot Module** (`crates/outpost-client/src/testing/screenshot.rs`)
   - `Screenshot` struct: stores width, height, and RGBA pixel buffer
   - `capture_from_image()`: captures from Bevy `Image` buffers (desktop and WASM variants)
   - `non_black_pixel_count()`: utility for validating non-empty renders

2. **Image Comparison** (`crates/outpost-client/src/testing/image_diff.rs`)
   - `pixel_diff()`: pixel-by-pixel comparison with tolerance for antialiasing
   - `ssim_rgba()`: Structural Similarity Index (SSIM) for perceptual comparison
   - `ImageDiffResult`: detailed comparison metrics

3. **Reference Store** (`crates/outpost-client/src/testing/reference_store.rs`)
   - In-memory HashMap for storing reference images
   - `save()` and `get()` methods for reference management
   - Logs reference image loads for debugging

4. **Visual Assertions** (`crates/outpost-client/src/testing/assertions.rs`)
   - `assert_matches_reference()`: combines reference lookup, comparison, and diff visualization
   - `VisualAssertionResult`: detailed pass/fail results with metrics
   - Automatically saves diff images with red highlights on failure

### Test Structure

Visual tests live in `crates/outpost-client/tests/`:

- `visual_ui_panels.rs` - UI panel layout stability
- `visual_hex_grid.rs` - Hex grid rendering determinism
- `visual_charts.rs` - Chart rendering with distinct line colors
- `visual_scene_transitions.rs` - Scene transition correctness

## Creating a Visual Test

### Step 1: Generate Synthetic Screenshot

Create a deterministic pixel buffer representing your UI component:

```rust
use outpost_client::testing::screenshot::Screenshot;

fn generate_my_component_screenshot(width: u32, height: u32) -> Screenshot {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    
    // Draw deterministic patterns
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            
            // Example: blue header bar
            if y < height / 10 {
                pixels[idx] = 50;      // R
                pixels[idx + 1] = 100; // G
                pixels[idx + 2] = 200; // B
                pixels[idx + 3] = 255; // A
            } else {
                pixels[idx + 3] = 255; // Always set alpha
            }
        }
    }
    
    Screenshot { width, height, pixels }
}
```

**Key principles:**

- Use deterministic patterns (no randomness, no time-based rendering)
- Always set alpha channel to 255 for opaque pixels
- Use distinct colors for different UI zones (easier to debug)
- Keep dimensions reasonable (100x80 typical for tests)

### Step 2: Write the Test

```rust
use outpost_client::testing::assertions::assert_matches_reference;
use outpost_client::testing::reference_store::ReferenceStore;
use tracing::info;

#[test]
fn my_component_renders_correctly() {
    let mut store = ReferenceStore::new();
    let screenshot = generate_my_component_screenshot(100, 80);
    
    // Save as reference on first run
    store.save("my_component_v1", screenshot.clone());
    
    // Verify against reference
    let result = assert_matches_reference(
        &mut store,
        "my_component_v1",
        &screenshot,
        0  // tolerance: 0 for pixel-perfect, higher for antialiasing
    );
    
    info!("My component test: {}", if result.passed { "pass" } else { "fail" });
    assert!(result.passed, "Component rendering changed! Check diff image.");
}
```

### Step 3: Run the Test

```bash
# Desktop tests
cargo test -p outpost-client --test visual_my_component -- --nocapture

# With logging
RUST_LOG=info cargo test -p outpost-client --test visual_my_component -- --nocapture
```

## Updating References

When you **intentionally** change UI rendering and need to update references:

1. **Review the diff image** first to ensure the change is expected
   - Diff images saved as `{reference_name}__diff.png` in the target directory
   - Red pixels indicate differences

2. **Update the reference** by regenerating the screenshot:

   ```rust
   // In your test:
   let new_screenshot = generate_my_component_screenshot(100, 80);
   store.save("my_component_v2", new_screenshot.clone());  // New version
   ```

3. **Increment version numbers** in reference names when making breaking changes:
   - `my_component_v1` → `my_component_v2`
   - Helps track visual evolution over time

4. **Commit both code and reference updates** together

## Interpreting Diffs

### Pixel Diff

`pixel_diff(tolerance)` compares RGBA values channel-by-channel:

- **tolerance=0**: Pixel-perfect comparison (no differences allowed)
- **tolerance=5**: Allows minor antialiasing differences
- **tolerance=20**: Allows significant color variations

Returns `ImageDiffResult` with:

- `different_pixels`: count of pixels exceeding tolerance
- `total_pixels`: total pixel count
- `diff_percentage`: percentage of differing pixels

### SSIM (Structural Similarity)

`ssim_rgba()` computes perceptual similarity:

- **1.0**: Identical images
- **0.9-1.0**: Very similar (minor differences)
- **0.5-0.9**: Noticeable differences
- **<0.5**: Significantly different

Uses luminance-based comparison with constants:

- C1 = 0.01² (variance stabilization)
- C2 = 0.03² (contrast stabilization)

**When to use each:**

- **pixel_diff**: Exact rendering validation (UI layouts, colors)
- **ssim**: Perceptual comparison (charts, gradients, natural images)

### Diff Images

When `assert_matches_reference()` fails, it saves a diff image:

- **Location**: `target/debug/{reference_name}__diff.png`
- **Red pixels**: Differences detected
- **Original colors**: Matching pixels

**Debugging workflow:**

1. Test fails
2. Open `{reference_name}__diff.png`
3. Identify red-highlighted areas
4. Investigate why those pixels changed
5. Fix code or update reference as appropriate

## CI Integration

### GitHub Actions Workflow

Visual tests run in CI via `.github/workflows/visual_tests.yml`:

**Desktop Tests (Linux + xvfb):**

- Uses X virtual framebuffer for headless rendering
- Runs all visual test suites: `cargo test -p outpost-client --test visual_*`
- Uploads diff images as artifacts on failure (7-day retention)

**WASM Tests (headless Chrome):**

- Builds WASM targets with `wasm-pack`
- Executes in headless Chrome browser
- Also uploads diff images on failure

**Trigger conditions:**

- Push to `main` or `claude/**` branches
- All PRs targeting `main`

### Viewing CI Failures

1. Go to GitHub Actions tab in repository
2. Click failed workflow run
3. Navigate to "Artifacts" section
4. Download `desktop-visual-diffs` or `wasm-visual-diffs`
5. Extract and review `*__diff.png` files

### Local CI Simulation

Simulate CI environment locally:

```bash
# Desktop (requires xvfb on Linux)
xvfb-run -a cargo test -p outpost-client --test visual_ui_panels -- --nocapture

# WASM (requires wasm-pack)
cd crates/outpost-client
wasm-pack build --test visual_ui_panels --target web --dev
```

## Best Practices

### Test Design

1. **Keep tests deterministic**
   - No random values
   - No time-based rendering
   - No external dependencies

2. **Use distinct visual patterns**
   - Different colors for different UI zones
   - Clear boundaries between elements
   - High contrast for easy debugging

3. **Test small components**
   - 100x80 pixels typical
   - Focus on specific UI elements
   - Faster comparisons, easier debugging

4. **Version your references**
   - Use `_v1`, `_v2` suffixes
   - Documents visual evolution
   - Helps with rollbacks

### Performance

1. **Synthetic rendering is fast**
   - No graphics backend required
   - No window creation overhead
   - Parallel test execution friendly

2. **Optimize pixel loops**
   - Pre-allocate pixel buffer
   - Use direct indexing, not iterators (when performance critical)
   - Consider SIMD for hot paths (future)

3. **Cache reference images**
   - ReferenceStore is in-memory
   - Reuse across multiple assertions
   - No disk I/O during comparison

### Debugging Failed Tests

1. **Check the diff image first**
   - Location: `target/debug/{reference_name}__diff.png`
   - Red = differences, original colors = matches

2. **Increase logging**

   ```bash
   RUST_LOG=debug cargo test -p outpost-client --test visual_* -- --nocapture
   ```

3. **Isolate the test**

   ```bash
   cargo test -p outpost-client --test visual_ui_panels specific_test_name
   ```

4. **Compare metrics**
   - `ImageDiffResult::diff_percentage` - how much changed?
   - `ssim_rgba()` score - perceptual similarity
   - Specific pixel coordinates from logs

## Advanced Patterns

### Multi-State Testing

Test different states of the same component:

```rust
#[test]
fn button_states_render_correctly() {
    let mut store = ReferenceStore::new();
    
    for state in &["default", "hover", "pressed", "disabled"] {
        let screenshot = generate_button_screenshot(state, 60, 30);
        let reference_name = format!("button_{}_v1", state);
        
        store.save(&reference_name, screenshot.clone());
        let result = assert_matches_reference(&mut store, &reference_name, &screenshot, 0);
        
        assert!(result.passed, "Button state '{}' rendering changed", state);
    }
}
```

### Tolerance Testing

For antialiased content, use higher tolerance:

```rust
#[test]
fn chart_with_smooth_lines() {
    let mut store = ReferenceStore::new();
    let screenshot = generate_chart_with_antialiasing(200, 150);
    
    store.save("chart_v1", screenshot.clone());
    
    // tolerance=5 allows minor antialiasing differences
    let result = assert_matches_reference(&mut store, "chart_v1", &screenshot, 5);
    
    assert!(result.passed);
}
```

### Negative Testing

Verify that different components DON'T match:

```rust
#[test]
fn scenes_are_visually_distinct() {
    let start_menu = generate_scene_screenshot("start_menu", 100, 80);
    let gameplay = generate_scene_screenshot("gameplay", 100, 80);
    
    assert_ne!(start_menu.pixels, gameplay.pixels, 
               "Start menu and gameplay should have different visuals");
}
```

## Troubleshooting

### Common Issues

**"Reference not found"**

- Ensure `store.save()` is called before `assert_matches_reference()`
- Check reference name spelling (case-sensitive)

**"Test fails but diff image shows no differences"**

- Check alpha channel values (must be 255 for opaque)
- Verify pixel buffer size matches width × height × 4

**"Tests pass locally but fail in CI"**

- Ensure deterministic rendering (no randomness)
- Check for platform-specific rendering differences
- Review CI logs for environment differences

**"WASM tests don't run"**

- Install wasm-pack: `curl https://rustwasm.org/wasm-pack/installer/init.sh -sSf | sh`
- Add `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Use `wasm-pack build --test` instead of `cargo test`

### Performance Issues

**Tests run slowly**

- Reduce screenshot dimensions
- Use `cargo test --release` for faster execution
- Parallelize independent tests (default with `cargo test`)

**Large diff images**

- Diff images are full-size PNG files
- Only saved on failure (not stored in version control)
- Clean up: `rm target/debug/**/*__diff.png`

## Examples

See existing visual tests for complete examples:

- [visual_ui_panels.rs](../crates/outpost-client/tests/visual_ui_panels.rs) - Basic UI layout
- [visual_hex_grid.rs](../crates/outpost-client/tests/visual_hex_grid.rs) - Grid patterns
- [visual_charts.rs](../crates/outpost-client/tests/visual_charts.rs) - Charts with color validation
- [visual_scene_transitions.rs](../crates/outpost-client/tests/visual_scene_transitions.rs) - Multi-scene testing

## Future Enhancements

Potential improvements to the visual testing system:

- [ ] Reference image versioning system (git-based)
- [ ] Automatic reference updates via CLI flag
- [ ] Visual diff viewer web UI
- [ ] SIMD-accelerated pixel comparison
- [ ] Perceptual hashing for fuzzy matching
- [ ] Integration with property-based testing (proptest)
- [ ] Screenshot capture from actual Bevy rendering (optional)
