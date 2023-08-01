use crate::math::{EnhancedVector, Vector3};
use crate::ray::Ray;

pub trait Environment {
    fn color(&self, ray: &Ray) -> Vector3;
}

pub struct Black {}

impl Environment for Black {
    fn color(&self, _ray: &Ray) -> Vector3 {
        Vector3::zero()
    }
}

pub struct Gradient {
    colors: (Vector3, Vector3),
}

impl Gradient {
    pub fn new(from: Vector3, to: Vector3) -> Self {
        Self { colors: (from, to) }
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Self {
            colors: (Vector3::one(), Vector3::new(0.5, 0.7, 1.0)),
        }
    }
}

impl Environment for Gradient {
    fn color(&self, ray: &Ray) -> Vector3 {
        let unit_direction = ray.direction.unit();
        let t = 0.5 * (unit_direction.y + 1.);
        self.colors.0 * (1.0 - t) + self.colors.1 * t
    }
}
