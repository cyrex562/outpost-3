use bevy::prelude::*;

/// System to control the 3D camera for viewing the solar system
pub fn camera_controller(
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Camera2d>)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for mut transform in camera_query.iter_mut() {
        let mut movement = Vec3::ZERO;
        let rotation_speed = 0.5;
        let zoom_speed = 50.0;

        // Camera movement
        if keyboard_input.pressed(KeyCode::KeyW) {
            movement += transform.forward();
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            movement -= transform.forward();
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            movement -= transform.right();
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            movement += transform.right();
        }
        if keyboard_input.pressed(KeyCode::KeyQ) {
            movement += transform.up();
        }
        if keyboard_input.pressed(KeyCode::KeyE) {
            movement -= transform.up();
        }

        // Apply movement
        if movement.length() > 0.0 {
            transform.translation += movement.normalize() * 100.0 * time.delta_seconds();
        }

        // Camera rotation with mouse (simplified)
        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            transform.rotate_y(rotation_speed * time.delta_seconds());
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            transform.rotate_y(-rotation_speed * time.delta_seconds());
        }
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            transform.rotate_local_x(rotation_speed * time.delta_seconds());
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            transform.rotate_local_x(-rotation_speed * time.delta_seconds());
        }

        // Zoom in/out
        if keyboard_input.pressed(KeyCode::Equal) || keyboard_input.pressed(KeyCode::NumpadAdd) {
            transform.translation += transform.forward() * zoom_speed * time.delta_seconds();
        }
        if keyboard_input.pressed(KeyCode::Minus) || keyboard_input.pressed(KeyCode::NumpadSubtract) {
            transform.translation -= transform.forward() * zoom_speed * time.delta_seconds();
        }
    }
}
