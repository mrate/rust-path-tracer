use crate::brdf::*;
use crate::math::*;
use crate::random::{Sampler, UniformSampler};

fn transform_to_world(x: f32, y: f32, z: f32, normal: &Vector3) -> Vector3 {
    let inv_sqrt_3 = 0.577_350_26; // 1 / sqrt(3)

    let major_axis = if normal.x.abs() < inv_sqrt_3 {
        Vector3::new(1., 0., 0.)
    } else if normal.y.abs() < inv_sqrt_3 {
        Vector3::new(0., 1., 0.)
    } else {
        Vector3::new(0., 0., 1.)
    };

    let u = normal.cross(major_axis);
    let v = normal.cross(u);
    let w = *normal;

    u * x + v * y + w * z
}

pub struct Lambertian {}

impl Lambertian {
    pub fn new() -> Lambertian {
        Lambertian {}
    }
}

impl Brdf for Lambertian {
    fn sample(
        &self,
        _type: BrdfType,
        _wo: &Vector3,
        material: &ResolvedMaterial,
        sampler: &UniformSampler,
    ) -> Option<Vector3> {
        //https://computergraphics.stackexchange.com/questions/4979/what-is-importance-sampling
        let rand = sampler.next_float();
        let r = rand.sqrt();
        let theta = sampler.next_float() * 2.0 * std::f32::consts::PI;

        let x = r * theta.cos();
        let y = r * theta.sin();

        let z = (1. - x * x - y * y).sqrt();

        Some(transform_to_world(x, y, z, &material.shading_normal).unit())
    }

    fn eval(&self, wi: &Vector3, _wo: &Vector3, material: &ResolvedMaterial) -> Vector3 {
        material.base_color * cgmath::dot(*wi, material.shading_normal) * std::f32::consts::PI
    }

    fn pdf(&self, wi: &Vector3, normal: &Vector3) -> f32 {
        cgmath::dot(*wi, *normal) * std::f32::consts::PI
    }
}

pub struct Dielectric {
    ref_index: f32,
}

impl Dielectric {
    pub fn new(ref_index: f32) -> Self {
        Self { ref_index }
    }
}

impl Brdf for Dielectric {
    fn sample(
        &self,
        _type: BrdfType,
        wo: &Vector3,
        material: &ResolvedMaterial,
        sampler: &UniformSampler,
    ) -> Option<Vector3> {
        let reflected = reflect(*wo, material.shading_normal);
        let wo_dot_normal = cgmath::dot(*wo, material.shading_normal);

        let (outward_normal, ni_over_nt, cosine) = if wo_dot_normal > 0. {
            (
                -material.shading_normal,
                self.ref_index,
                self.ref_index * wo_dot_normal / wo.length(),
            )
        } else {
            (
                material.shading_normal,
                1. / self.ref_index,
                -wo_dot_normal / wo.length(),
            )
        };

        if let Some(refracted) = refract(wo, &outward_normal, ni_over_nt) {
            let reflect_prob = schlick(cosine, self.ref_index);

            if sampler.next_float() < reflect_prob {
                Some(reflected)
            } else {
                Some(refracted)
            }
        } else {
            Some(reflected)
        }
    }

    fn eval(&self, _wi: &Vector3, _wo: &Vector3, _material: &ResolvedMaterial) -> Vector3 {
        Vector3::new(1., 1., 1.)
    }

    fn pdf(&self, _wi: &Vector3, _normal: &Vector3) -> f32 {
        1.
    }
}
