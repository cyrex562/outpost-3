use std::ops::RangeInclusive;
use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq)]
pub struct CameraLimits {
    pub world_x: RangeInclusive<f32>,
    pub world_y: RangeInclusive<f32>,
    pub zoom: RangeInclusive<f32>,
}

impl CameraLimits {
    pub fn new(world_x: RangeInclusive<f32>, world_y: RangeInclusive<f32>, zoom: RangeInclusive<f32>) -> Self {
        Self { world_x, world_y, zoom }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CameraState {
    pub x: f32,
    pub y: f32,
    /// Zoom factor where 1.0 = default. Smaller = zoom in for orthographic when using scale.
    pub zoom: f32,
    pub pan_speed: f32,
    pub zoom_step: f32,
    pub limits: CameraLimits,
}

impl CameraState {
    pub fn new(limits: CameraLimits) -> Self {
        Self { x: 0.0, y: 0.0, zoom: 1.0, pan_speed: 800.0, zoom_step: 0.1, limits }
    }

    pub fn with_params(mut self, pan_speed: f32, zoom_step: f32) -> Self {
        self.pan_speed = pan_speed;
        self.zoom_step = zoom_step;
        self
    }

    pub fn clamp(&self) -> Self {
        let mut s = self.clone();
        s.x = s.x.clamp(*s.limits.world_x.start(), *s.limits.world_x.end());
        s.y = s.y.clamp(*s.limits.world_y.start(), *s.limits.world_y.end());
        s.zoom = s.zoom.clamp(*s.limits.zoom.start(), *s.limits.zoom.end());
        s
    }

    /// Apply keyboard axes: (-1..1, -1..1) for x/y where +x is right, +y is up.
    pub fn apply_keyboard(&self, axis_x: f32, axis_y: f32, dt: f32) -> Self {
        let mut s = self.clone();
        s.x += axis_x * s.pan_speed * dt;
        s.y += axis_y * s.pan_speed * dt;
        s.clamp()
    }

    /// Apply mouse drag in pixels converted to world units by sensitivity.
    /// drag_dx positive means dragging mouse to the right, we pan camera to the left (world right), hence subtract.
    pub fn apply_mouse_drag(&self, drag_dx: f32, drag_dy: f32, sensitivity: f32) -> Self {
        let mut s = self.clone();
        s.x -= drag_dx * sensitivity;
        s.y += drag_dy * sensitivity; // typical screen Y-down, so invert
        s.clamp()
    }

    /// Apply scroll wheel delta where positive zooms in (decrease scale) if using Transform scale.
    pub fn apply_wheel(&self, wheel_y: f32) -> Self {
        let mut s = self.clone();
        s.zoom *= 1.0 - wheel_y * s.zoom_step;
        // guard against NaN/inf
        if !s.zoom.is_finite() {
            s.zoom = 1.0;
        }
        s.clamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn limits() -> CameraLimits {
        CameraLimits::new(-1000.0..=1000.0, -1000.0..=1000.0, 0.25..=4.0)
    }

    #[test]
    fn keyboard_pan_respects_clamp() {
        let s = CameraState::new(limits()).with_params(10_000.0, 0.1);
        let s2 = s.apply_keyboard(1.0, 1.0, 1.0);
        assert_eq!(s2.x, 1000.0);
        assert_eq!(s2.y, 1000.0);
    }

    #[test]
    fn mouse_drag_inverts_y_and_clamps() {
        let s = CameraState::new(limits());
        let s2 = s.apply_mouse_drag(2000.0, -3000.0, 1.0);
        // x decreases due to drag right
        assert_eq!(s2.x, -1000.0);
        // y decreases due to negative dy but inverted sign then clamped
        assert_eq!(s2.y, -1000.0);
    }

    #[test]
    fn wheel_zoom_clamped_and_finite() {
        let s = CameraState::new(limits()).with_params(800.0, 10.0);
        let s2 = s.apply_wheel(1.0);
        // 1 - 1*10 = -9, so zoom *= -9 => -9 -> clamped to 0.25
        assert_eq!(s2.zoom, 0.25);
    }

    proptest! {
        #[test]
        fn zoom_always_within_limits(wheel in -10.0f32..10.0) {
            let s = CameraState::new(limits());
            let s2 = s.apply_wheel(wheel);
            prop_assert!(s2.zoom >= *s.limits.zoom.start() && s2.zoom <= *s.limits.zoom.end());
        }

        #[test]
        fn clamping_idempotent(x in -10_000.0f32..10_000.0, y in -10_000.0f32..10_000.0, z in -100.0f32..100.0) {
            let mut s = CameraState::new(limits());
            s.x = x; s.y = y; s.zoom = z;
            let once = s.clamp();
            let twice = once.clamp();
            prop_assert_eq!(once, twice);
        }
    }
}
