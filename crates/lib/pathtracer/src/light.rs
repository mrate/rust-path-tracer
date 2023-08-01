use crate::math::{smoothstep, EnhancedVector, Vector3};

pub trait Attenuable {
    fn intensity_at(&self, position: &Vector3) -> Vector3;
}

pub struct Directional {
    pub dir: Vector3,
    pub color: Vector3,
    pub intensity: f32,
}

impl Attenuable for Directional {
    fn intensity_at(&self, _position: &Vector3) -> Vector3 {
        self.color * self.intensity
    }
}

pub struct Point {
    pub position: Vector3,
    pub color: Vector3,
    pub intensity: f32,
    pub range: f32,
    pub range_squared: f32,
}

impl Attenuable for Point {
    fn intensity_at(&self, position: &Vector3) -> Vector3 {
        // TODO: Exp. fallof

        // TODO: Distance

        self.color
            * self.intensity
            * (1.0
                - smoothstep(
                    self.range * 0.75,
                    self.range,
                    (self.position - *position).length(),
                ))
    }
}

pub enum Light {
    Directional(Directional),
    Point(Point),
}

impl Light {
    pub fn direction_distance_from(&self, position: &Vector3) -> (Vector3, f32) {
        match self {
            Light::Directional(dir) => (dir.dir, std::f32::INFINITY),
            Light::Point(point) => {
                let dir = point.position - *position;
                (dir.unit(), dir.length())
            }
        }
    }
}

impl Attenuable for Light {
    fn intensity_at(&self, position: &Vector3) -> Vector3 {
        match self {
            Light::Directional(dir) => dir.intensity_at(position),
            Light::Point(point) => point.intensity_at(position),
        }
    }
}
