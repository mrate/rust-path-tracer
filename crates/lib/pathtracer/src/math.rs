use crate::consts::*;
use crate::material::Material;
use crate::random::{Sampler, UniformSampler};

use std::ops::{Add, Mul};
use std::sync::Arc;

use cgmath::*;

use rand::prelude::*;

pub type Vector3 = cgmath::Vector3<f32>;
pub type Quaternion = cgmath::Quaternion<f32>;
pub type Matrix3 = cgmath::Matrix3<f32>;
pub type Matrix4 = cgmath::Matrix4<f32>;

pub const TWO_PI: f32 = 2. * std::f32::consts::PI;
pub const ONE_OVER_PI: f32 = 1. / std::f32::consts::PI;

#[inline]
pub fn to_v3(v: f32) -> Vector3 {
    Vector3::new(v, v, v)
}

#[inline]
pub fn add(lhs: Vector3, rhs: f32) -> Vector3 {
    Vector3::new(lhs.x + rhs, lhs.y + rhs, lhs.z + rhs)
}

#[inline]
pub fn random_min_max(min: f32, max: f32) -> f32 {
    min + random::<f32>() * (max - min)
}

#[inline]
pub fn clamp(v: f32, min: f32, max: f32) -> f32 {
    v.min(max).max(min)
}

#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // Scale, bias and saturate x to 0..1 range
    let x = clamp((x - edge0) / (edge1 - edge0), 0., 1.);
    // Evaluate polynomial
    x * x * (3. - 2. * x)
}

#[inline]
pub fn saturate(value: f32) -> f32 {
    clamp(value, 0., 1.)
}

#[inline]
pub fn luminance(rgb: Vector3) -> f32 {
    rgb.dot(LUMINANCE)
}

#[inline]
pub fn lerp_scalar(v1: f32, v2: f32, f: f32) -> f32 {
    (1. - f) * v1 + f * v2
}

#[inline]
pub fn lerp(v1: Vector3, v2: Vector3, f: f32) -> Vector3 {
    Vector3::new(
        lerp_scalar(v1.x, v2.x, f),
        lerp_scalar(v1.y, v2.y, f),
        lerp_scalar(v1.z, v2.z, f),
    )
}

pub trait EnhancedVector<T> {
    fn length(&self) -> T;
    fn squared_length(&self) -> T;
    fn unit(&self) -> cgmath::Vector3<T>;
    fn min(&self, rhs: cgmath::Vector3<T>) -> cgmath::Vector3<T>;
    fn max(&self, rhs: cgmath::Vector3<T>) -> cgmath::Vector3<T>;
    fn zero() -> cgmath::Vector3<T>;
    fn one() -> cgmath::Vector3<T>;
    fn from_slice(values: &[T]) -> cgmath::Vector3<T>;
    fn mul(&self, rhs: cgmath::Vector3<T>) -> cgmath::Vector3<T>;
    fn clamp(&self, min: T, max: T) -> cgmath::Vector3<T>;
}

impl EnhancedVector<f32> for Vector3 {
    #[inline]
    fn length(&self) -> f32 {
        self.squared_length().sqrt()
    }

    #[inline]
    fn squared_length(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline]
    fn unit(&self) -> Vector3 {
        let length = self.length();
        Vector3 {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
        }
    }

    #[inline]
    fn min(&self, rhs: Vector3) -> Vector3 {
        Vector3 {
            x: self.x.min(rhs.x),
            y: self.y.min(rhs.y),
            z: self.z.min(rhs.z),
        }
    }

    #[inline]
    fn max(&self, rhs: Vector3) -> Vector3 {
        Vector3 {
            x: self.x.max(rhs.x),
            y: self.y.max(rhs.y),
            z: self.z.max(rhs.z),
        }
    }

    #[inline]
    fn zero() -> Self {
        Vector3::new(0., 0., 0.)
    }

    #[inline]
    fn one() -> Self {
        Vector3::new(1., 1., 1.)
    }

    #[inline]
    fn from_slice(values: &[f32]) -> Vector3 {
        Vector3::new(values[0], values[1], values[2])
    }

    #[inline]
    fn mul(&self, rhs: Vector3) -> Vector3 {
        Vector3::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }

    #[inline]
    fn clamp(&self, min: f32, max: f32) -> Vector3 {
        Vector3::new(
            clamp(self.x, min, max),
            clamp(self.y, min, max),
            clamp(self.z, min, max),
        )
    }
}

// Sphere
pub struct Sphere {
    pub center: Vector3,
    pub radius: f32,
    pub material: Arc<Material>,
}

impl Sphere {
    pub fn position_radius(x: f32, y: f32, z: f32, radius: f32, material: Arc<Material>) -> Sphere {
        Sphere {
            center: Vector3 { x, y, z },
            radius,
            material,
        }
    }

    pub fn normal(&self, position: Vector3) -> Vector3 {
        (position - self.center).unit()
    }
}

pub struct Average {
    spp: i32,
    prev_pass_add: f32,
    this_pass_add: f32,
}

impl Default for Average {
    fn default() -> Self {
        Self {
            spp: 0,
            prev_pass_add: 0.0,
            this_pass_add: 0.0,
        }
    }
}

impl Average {
    pub fn sample(&self) -> i32 {
        self.spp
    }

    pub fn set_sample(&mut self, sample: i32) {
        self.spp = sample
    }

    pub fn reset(&mut self) {
        self.spp = 0;
    }

    pub fn next_frame(&mut self) {
        self.prev_pass_add = (self.spp as f32) / (self.spp + 1) as f32;
        self.this_pass_add = 1.0 / (self.spp + 1) as f32;
        self.spp += 1;
    }

    pub fn average<U>(&self, prev: U, new: U) -> <<U as Mul<f32>>::Output as Add>::Output
    where
        U: Mul<f32>,
        <U as Mul<f32>>::Output: Add,
    {
        let prev_value = prev
            * match self.spp {
                1 => 0.,
                _ => self.prev_pass_add,
            };

        new * self.this_pass_add + prev_value
    }
}

pub fn schlick(cosine: f32, ref_index: f32) -> f32 {
    let r0 = (1. - ref_index) / (1. + ref_index);
    let r0 = r0 * r0;
    r0 + (1. - r0) * (1. - cosine).powf(5.)
}

pub fn reflect(dir: Vector3, normal: Vector3) -> Vector3 {
    dir - 2.0 * cgmath::dot(dir, normal) * normal
}

pub fn refract(dir: &Vector3, normal: &Vector3, ni_over_nt: f32) -> Option<Vector3> {
    let unit = dir.unit();
    let dt = cgmath::dot(unit, *normal);
    let discriminant = 1. - ni_over_nt * ni_over_nt * (1. - dt * dt);

    if discriminant > 0. {
        Some(ni_over_nt * (unit - normal * dt) - normal * discriminant.sqrt())
    } else {
        None
    }
}

// Source: https://github.com/boksajak/referencePT/blob/master/shaders/brdf.h

pub fn base_color_to_specular_f0(base_color: Vector3, metalness: f32) -> Vector3 {
    lerp(MIN_DIELECTRICS_F0_VEC, base_color, metalness)
}

pub fn base_color_to_diffuse_reflectance(base_color: Vector3, metalness: f32) -> Vector3 {
    base_color * (1. - metalness)
}

// Schlick's approximation to Fresnel term
// f90 should be 1.0, except for the trick used by Schuler (see 'shadowedF90' function)
pub fn eval_fresnel(f0: Vector3, f90: Vector3, n_dot_s: f32) -> Vector3 {
    f0 + (f90 - f0) * (1. - n_dot_s).powf(5.)
}

// Attenuates F90 for very low F0 values
// Source: "An efficient and Physically Plausible Real-Time Shading Model" in ShaderX7 by Schuler
// Also see section "Overbright highlights" in Hoffman's 2010 "Crafting Physically Motivated Shading Models for Game Development" for discussion
// IMPORTANT: Note that when F0 is calculated using metalness, it's value is never less than MIN_DIELECTRICS_F0, and therefore,
// this adjustment has no effect. To be effective, F0 must be authored separately, or calculated in different way. See main text for discussion.
pub fn shadowed_f90(f0: Vector3) -> f32 {
    // This scaler value is somewhat arbitrary, Schuler used 60 in his article. In here, we derive it from MIN_DIELECTRICS_F0 so
    // that it takes effect for any reflectance lower than least reflective dielectrics
    //const float t = 60.0f;
    let t = 1. / MIN_DIELECTRICS_F0;
    (t * luminance(f0)).min(1.)
}

// Calculates rotation quaternion from input vector to the vector (0, 0, 1)
// Input vector must be normalized!
pub fn get_rotation_to_z_axis(input: Vector3) -> Quaternion {
    // Handle special case when input is exact or near opposite of (0, 0, 1)
    if input.z < -0.99999 {
        Quaternion::new(0., 1., 0., 0.)
    } else {
        Quaternion::new(1. + input.z, input.y, -input.x, 0.).normalize()
    }
}

// Calculates rotation quaternion from vector (0, 0, 1) to the input vector
// Input vector must be normalized!
pub fn get_rotation_from_z_axis(input: Vector3) -> Quaternion {
    // Handle special case when input is exact or near opposite of (0, 0, 1)
    if input.z < -0.99999 {
        Quaternion::new(0., 1., 0., 0.)
    } else {
        Quaternion::new(-input.y, input.x, 0., 1. + input.z).normalize()
    }
}

// Returns the quaternion with inverted rotation
pub fn invert_rotation(q: Quaternion) -> Quaternion {
    Quaternion::new(q.s, -q.v.x, -q.v.y, -q.v.z)
}

// Optimized point rotation using quaternion
// Source: https://gamedev.stackexchange.com/questions/28395/rotating-vector3-by-a-quaternion
pub fn rotate_point(q: Quaternion, v: Vector3) -> Vector3 {
    let q_axis = q.v;

    2. * q_axis.dot(v) * q_axis + (q.s * q.s - q_axis.dot(q_axis)) * v + 2. * q.s * q_axis.cross(v)
}

// Samples a direction within a hemisphere oriented along +Z axis with a cosine-weighted distribution
// Source: "Sampling Transformations Zoo" in Ray Tracing Gems by Shirley et al.
pub fn sample_hemisphere(sampler: &UniformSampler) -> (Vector3, f32) {
    let (ux, uy) = (sampler.next_float(), sampler.next_float());

    let a = ux.sqrt();
    let b = TWO_PI * uy;

    // (Direction, pdf)
    let result = Vector3::new(a * b.cos(), a * b.sin(), (1. - ux).sqrt());

    (result, result.z * ONE_OVER_PI)
}
