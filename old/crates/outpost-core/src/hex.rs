//! Axial hex math utilities based on Red Blob Games reference

use core::ops::{Add, Sub};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}

impl Hex {
    #[inline]
    pub const fn new(q: i32, r: i32) -> Self { Self { q, r } }

    #[inline]
    pub const fn s(&self) -> i32 { -self.q - self.r }

    #[inline]
    pub const fn add(self, other: Hex) -> Hex { Hex::new(self.q + other.q, self.r + other.r) }

    #[inline]
    pub const fn sub(self, other: Hex) -> Hex { Hex::new(self.q - other.q, self.r - other.r) }

    #[inline]
    pub const fn scale(self, k: i32) -> Hex { Hex::new(self.q * k, self.r * k) }

    // Directions in axial coordinates (pointy-top): 0..5
    // Matches cube directions:
    // 0: (+1, 0, -1)
    // 1: (+1, -1, 0)
    // 2: (0, -1, +1)
    // 3: (-1, 0, +1)
    // 4: (-1, +1, 0)
    // 5: (0, +1, -1)
    #[inline]
    pub const fn dir(dir: usize) -> Hex {
        match dir % 6 {
            0 => Hex { q: 1, r: 0 },
            1 => Hex { q: 1, r: -1 },
            2 => Hex { q: 0, r: -1 },
            3 => Hex { q: -1, r: 0 },
            4 => Hex { q: -1, r: 1 },
            _ => Hex { q: 0, r: 1 },
        }
    }

    #[inline]
    pub fn neighbor(self, dir: usize) -> Hex { self.add(Hex::dir(dir)) }

    #[inline]
    pub fn neighbors(self) -> [Hex; 6] {
        [
            self.neighbor(0),
            self.neighbor(1),
            self.neighbor(2),
            self.neighbor(3),
            self.neighbor(4),
            self.neighbor(5),
        ]
    }

    #[inline]
    pub fn length(self) -> i32 { ((self.q.abs() + self.r.abs() + self.s().abs()) / 2) as i32 }

    #[inline]
    pub fn distance(self, other: Hex) -> i32 { self.sub(other).length() }

    // Rotate 60 degrees around origin; k steps positive = CCW
    #[inline]
    pub fn rotate_left(self) -> Hex { Hex::new(-self.s(), -self.q) }
    #[inline]
    pub fn rotate_right(self) -> Hex { Hex::new(-self.r, -self.s()) }
    #[inline]
    pub fn rotate(self, steps_ccw: i32) -> Hex {
        let mut h = self;
        let n = ((steps_ccw % 6) + 6) % 6;
        for _ in 0..n { h = h.rotate_left(); }
        h
    }

    // Returns all hexes at exact radius around center (excluding center when radius>0)
    pub fn ring(center: Hex, radius: i32) -> Vec<Hex> {
        if radius <= 0 { return vec![center]; }
        let mut results = Vec::with_capacity((radius * 6) as usize);
        // start at (center + dir(4)*radius)
        let mut h = center.add(Hex::dir(4).scale(radius));
        for side in 0..6 {
            for _ in 0..radius { // walk radius steps on each side
                results.push(h);
                h = h.neighbor((side + 0) % 6);
            }
        }
        results
    }

    // Line from self to target inclusive using cube lerp/round
    pub fn line_to(self, target: Hex) -> Vec<Hex> {
        let n = self.distance(target);
        if n == 0 { return vec![self]; }
        let a = self.to_cube_f();
        let b = target.to_cube_f();
        let mut out = Vec::with_capacity((n + 1) as usize);
        for i in 0..=n {
            let t = (i as f32) / (n as f32);
            out.push(Hex::from_cube_f(lerp_tuple(a, b, t)));
        }
        out
    }

    #[inline]
    fn to_cube_f(self) -> (f32, f32, f32) {
        let x = self.q as f32;
        let z = self.r as f32;
        let y = -x - z;
        (x, y, z)
    }

    #[inline]
    fn from_cube_f((x, y, z): (f32, f32, f32)) -> Hex { cube_round(x, y, z) }
}

#[inline]
fn lerp_tuple(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

#[inline]
fn cube_round(x: f32, y: f32, z: f32) -> Hex {
    let mut rx = x.round();
    let mut ry = y.round();
    let mut rz = z.round();

    let dx = (rx - x).abs();
    let dy = (ry - y).abs();
    let dz = (rz - z).abs();

    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dy > dz {
        ry = -rx - rz;
    } else {
        rz = -rx - ry;
    }

    // convert cube (x,z-> axial q,r)
    Hex::new(rx as i32, rz as i32)
}

impl Add for Hex {
    type Output = Hex;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output { self.add(rhs) }
}

impl Sub for Hex {
    type Output = Hex;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output { self.sub(rhs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn neighbors_are_distance_one() {
        let h = Hex::new(3, -2);
        for n in h.neighbors() { assert_eq!(h.distance(n), 1); }
    }

    #[test]
    fn ring_size_and_radius() {
        for r in 1..6 {
            let c = Hex::new(0, 0);
            let ring = Hex::ring(c, r);
            assert_eq!(ring.len() as i32, 6 * r);
            for h in ring { assert_eq!(c.distance(h), r); }
        }
    }

    #[test]
    fn line_is_continuous() {
        let a = Hex::new(0, 0);
        let b = Hex::new(5, -2);
        let path = a.line_to(b);
        for w in path.windows(2) { assert_eq!(w[0].distance(w[1]), 1); }
        assert_eq!(*path.first().unwrap(), a);
        assert_eq!(*path.last().unwrap(), b);
    }

    proptest! {
        #[test]
        fn distance_properties(q1 in -50i32..50, r1 in -50i32..50, q2 in -50i32..50, r2 in -50i32..50) {
            let a = Hex::new(q1, r1);
            let b = Hex::new(q2, r2);
            prop_assert_eq!(a.distance(a), 0);
            prop_assert_eq!(a.distance(b), b.distance(a));
        }

        #[test]
        fn ring_count_and_radius(center_q in -10i32..10, center_r in -10i32..10, radius in 0i32..8) {
            let c = Hex::new(center_q, center_r);
            let ring = Hex::ring(c, radius);
            if radius == 0 {
                prop_assert_eq!(ring, vec![c]);
            } else {
                prop_assert_eq!(ring.len() as i32, 6*radius);
                for h in ring { prop_assert_eq!(c.distance(h), radius); }
            }
        }

        #[test]
        fn rotation_invariant(q in -20i32..20, r in -20i32..20) {
            let h = Hex::new(q,r);
            let mut x = h;
            for _ in 0..6 { x = x.rotate_left(); }
            prop_assert_eq!(x, h);
        }
    }
}
