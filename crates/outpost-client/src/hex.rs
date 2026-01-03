use bevy::prelude::Vec2;

const SQRT_3: f32 = 1.7320508075688772_f32;

// Axial (q, r) to world coordinates (pointy-top orientation)
pub fn axial_to_world(q: i32, r: i32, radius: f32) -> Vec2 {
    let qf = q as f32;
    let rf = r as f32;
    let x = radius * (SQRT_3 * qf + (SQRT_3 / 2.0) * rf);
    let y = radius * (1.5 * rf);
    Vec2::new(x, y)
}

// World coordinates to axial (q, r) rounded to nearest hex (pointy-top)
pub fn world_to_axial_round(world: Vec2, radius: f32) -> (i32, i32) {
    let x = world.x;
    let y = world.y;
    // inverse of axial_to_world (pointy-top)
    let qf = (SQRT_3 / 3.0 * x - 1.0 / 3.0 * y) / radius;
    let rf = (2.0 / 3.0 * y) / radius;
    let (q, r) = cube_round_from_axial(qf, rf);
    (q, r)
}

fn cube_round_from_axial(qf: f32, rf: f32) -> (i32, i32) {
    let xf = qf;
    let zf = rf;
    let yf = -xf - zf;

    let mut rx = xf.round();
    let mut ry = yf.round();
    let mut rz = zf.round();

    let dx = (rx - xf).abs();
    let dy = (ry - yf).abs();
    let dz = (rz - zf).abs();

    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dy > dz {
        ry = -rx - rz;
    } else {
        rz = -rx - ry;
    }

    (rx as i32, rz as i32)
}
