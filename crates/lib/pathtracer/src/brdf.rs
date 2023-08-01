use crate::material::*;
use crate::math::{EnhancedVector, Vector3};
use crate::random::UniformSampler;

use cgmath::Matrix3;

use std::sync::Arc;

pub struct ResolvedMaterial {
    pub base_color: Vector3,
    pub emissive: Vector3,
    pub geometry_normal: Vector3, // Cross product of vertex edges.
    pub shading_normal: Vector3, // Interpolation between vertex normals (flat shading + normal map).
    pub metalness: f32,
    pub roughness: f32,
    pub single_sided: bool,
}

pub struct Hit {
    pub position: Vector3,
    pub material: Arc<Material>,
    pub t: f32,
    pub uv: (f32, f32),
    pub normal: Vector3,
    pub tangent: Vector3,
    pub bitangent: Vector3,
}

impl Hit {
    pub fn resolve_material(&self) -> ResolvedMaterial {
        let base_color = self.material.base_color(self.uv);
        let emissive = self.material.emissive_color(self.uv);
        // TODO: tangent space...
        let geometry_normal = self.normal;

        // TODO: Tangent space, interpolate.
        let shading_normal = match self.material.has_normal() {
            true => {
                let btn = Matrix3::from_cols(self.bitangent, self.tangent, self.normal);
                let mat_norm = self.material.normal(self.uv);
                (btn * mat_norm).unit()
            }
            false => self.normal,
        };

        let metalness = self.material.metalness(self.uv);
        let roughness = self.material.roughness(self.uv);
        let single_sided = self.material.single_sided;

        ResolvedMaterial {
            base_color,
            emissive,
            geometry_normal,
            shading_normal,
            metalness,
            roughness,
            single_sided,
        }
    }
}

pub enum BrdfType {
    Diffuse,
    Specular,
}

pub trait Brdf {
    /// Returns an incident vector wi given outgoing vector wo and normal at incident point.
    fn sample(
        &self,
        brdf_type: BrdfType,
        wo: &Vector3,
        material: &ResolvedMaterial,
        sampler: &UniformSampler,
    ) -> Option<Vector3>;

    /// Returns attenuation given incident vector wi, outgoing vector wo, normal at incident point and hit record.
    fn eval(&self, wi: &Vector3, wo: &Vector3, material: &ResolvedMaterial) -> Vector3;

    /// Returns probabilistic density function of material.
    fn pdf(&self, wi: &Vector3, normal: &Vector3) -> f32;

    /// Return probability of selecting specular and diffuse BRDF.
    fn probability(&self, _v: &Vector3, _material: &ResolvedMaterial) -> f32 {
        0.5
    }
}
