use crate::math::*;
use crate::random::*;
use crate::ray::*;

pub trait Camera {
    fn ray(&self, x: f32, y: f32, sampler: &UniformSampler) -> Ray;
}

pub struct SimpleCamera {
    position: Vector3,
    lower_left: Vector3,
    horizontal: Vector3,
    vertical: Vector3,
}

impl SimpleCamera {
    pub fn look_at(
        position: Vector3,
        look_at: Vector3,
        up: Vector3,
        v_fov: f32,
        aspect_ratio: f32,
    ) -> Self {
        let tan = (v_fov.to_radians() / 2.0).tan();
        let height = 2.0 * tan;
        let width = aspect_ratio * height;

        let w = (position - look_at).unit();
        let u = up.cross(w).unit();
        let v = w.cross(u);

        let vertical = v * height;
        let horizontal = u * width;

        let lower_left = position - horizontal / 2.0 - vertical / 2.0 - w;

        Self {
            position,
            lower_left,
            horizontal,
            vertical,
        }
    }
}

impl Camera for SimpleCamera {
    fn ray(&self, x: f32, y: f32, _sampler: &UniformSampler) -> Ray {
        Ray {
            origin: self.position,
            direction: self.lower_left + self.horizontal * x + self.vertical * y - self.position,
        }
    }
}

pub struct ApertureCamera {
    pub position: Vector3,

    u: Vector3,
    v: Vector3,
    lens_radius: f32,
    lower_left: Vector3,
    horizontal: Vector3,
    vertical: Vector3,
}

impl ApertureCamera {
    pub fn look_at(
        position: Vector3,
        look_at: Vector3,
        up: Vector3,
        v_fov: f32,
        aspect_ratio: f32,
        aperture: f32,
        focus_distance: f32,
    ) -> Self {
        let tan = (v_fov.to_radians() / 2.0).tan();
        let height = 2.0 * tan;
        let width = aspect_ratio * height;

        let w = (position - look_at).unit();
        let u = up.cross(w).unit();
        let v = w.cross(u);

        let horizontal = focus_distance * u * width;
        let vertical = focus_distance * v * height;

        let lower_left = position - horizontal / 2.0 - vertical / 2.0 - focus_distance * w;
        let lens_radius = aperture / 2.;

        Self {
            position,
            u,
            v,
            lens_radius,
            lower_left,
            horizontal,
            vertical,
        }
    }
}

impl Camera for ApertureCamera {
    fn ray(&self, x: f32, y: f32, sampler: &UniformSampler) -> Ray {
        let random = self.lens_radius * unit_disk(sampler);
        let offset = self.u * random.x + self.v * random.y;

        Ray {
            origin: self.position + offset,
            direction: self.lower_left + self.horizontal * x + self.vertical * y
                - self.position
                - offset,
        }
    }
}
