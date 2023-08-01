use crate::brdf::*;
use crate::math::*;
use crate::random::*;

use cgmath::dot;

use crate::microfacet::*;

// Microfacet based BRDF.
// Source: https://github.com/boksajak/referencePT/blob/master/shaders/brdf.h
pub struct MicrofacetBrdf {}

impl MicrofacetBrdf {
    pub fn new() -> Self {
        Self {}
    }
}

impl Brdf for MicrofacetBrdf {
    fn sample(
        &self,
        brdf_type: BrdfType,
        wo: &Vector3,
        material: &ResolvedMaterial,
        sampler: &UniformSampler,
    ) -> Option<Vector3> {
        let v = -*wo;

        // Ignore incident ray coming from "below" the hemisphere
        if cgmath::dot(material.shading_normal, v) <= 0. {
            return None;
        }

        // Transform view direction into local space of our sampling routines
        // (local space is oriented so that its positive Z axis points along the shading normal)
        let q_rotation_to_z = get_rotation_to_z_axis(material.shading_normal);
        let v_local = rotate_point(q_rotation_to_z, v);
        let n_local = Vector3::new(0., 0., 1.);

        let (ray_direction_local, sample_weight) = match brdf_type {
            BrdfType::Diffuse => {
                // Sample diffuse ray using cosine-weighted hemisphere sampling
                let (ray_direction_local, _) = sample_hemisphere(sampler);
                let data = prepare_brdf_data(n_local, ray_direction_local, v_local, material);

                // Function 'diffuseTerm' is predivided by PDF of sampling the cosine weighted hemisphere
                let sample_weight = data.diffuse_reflectance; // diffuse_term(data) = 1.0 for Lambertian;

                // #if COMBINE_BRDFS_WITH_FRESNEL
                // Sample a half-vector of specular BRDF. Note that we're reusing random variable 'u' here, but correctly it should be an new independent random number
                let u = (sampler.next_float(), sampler.next_float());
                let h_specular = sample_ggx_vndf(v_local, (data.alpha, data.alpha), u);

                // Clamp HdotL to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
                let v_dot_h = 0.00001f32.max(1.0f32.min(dot(v_local, h_specular)));

                let diff = Vector3::one()
                    - eval_fresnel(
                        data.specular_f0,
                        to_v3(shadowed_f90(data.specular_f0)),
                        v_dot_h,
                    );

                (ray_direction_local, sample_weight.mul(diff))
                // #endif
            }
            BrdfType::Specular => {
                let data = prepare_brdf_data(
                    n_local,
                    Vector3::new(0., 0., 1.), /* unused L vector */
                    v_local,
                    material,
                );

                sample_specular_microfacet(
                    v_local,
                    data.alpha,
                    data.alpha_squared,
                    data.specular_f0,
                    (sampler.next_float(), sampler.next_float()),
                )
            }
        };

        // Prevent tracing direction with no contribution
        if luminance(sample_weight) == 0. {
            return None;
        }

        // Transform sampled direction Llocal back to V vector space
        let ray_direction =
            rotate_point(invert_rotation(q_rotation_to_z), ray_direction_local).unit();

        // Prevent tracing direction "under" the hemisphere (behind the triangle)
        if cgmath::dot(material.geometry_normal, ray_direction) <= 0. {
            return None;
        }

        Some(ray_direction)
        // None
    }

    fn eval(&self, wi: &Vector3, wo: &Vector3, material: &ResolvedMaterial) -> Vector3 {
        // Prepare data needed for BRDF evaluation - unpack material properties and evaluate commonly used terms (e.g. Fresnel, NdotL, ...)
        let n = material.shading_normal;

        let data = prepare_brdf_data(n, *wi, -*wo, material);

        // Ignore V and L rays "below" the hemisphere
        if data.v_backfacing || data.l_backfacing {
            return Vector3::zero();
        }

        // Eval specular and diffuse BRDFs
        let specular = eval_microfacet(&data);
        let diffuse = eval_lambertian(&data);

        // Combine specular and diffuse layers
        // #if COMBINE_BRDFS_WITH_FRESNEL
        // Specular is already multiplied by F, just attenuate diffuse
        (Vector3::one() - data.f).mul(diffuse) + specular
        // #else
        // 	return diffuse + specular;
        // #endif
    }

    fn pdf(&self, _wi: &Vector3, _normal: &Vector3) -> f32 {
        1.
    }

    fn probability(&self, v: &Vector3, material: &ResolvedMaterial) -> f32 {
        // Evaluate Fresnel term using the shading normal
        // Note: we use the shading normal instead of the microfacet normal (half-vector) for Fresnel term here. That's suboptimal for rough surfaces at grazing angles, but half-vector is yet unknown at this point
        let specular_f0 = luminance(base_color_to_specular_f0(
            material.base_color,
            material.metalness,
        ));
        let diffuse_reflectance = luminance(base_color_to_diffuse_reflectance(
            material.base_color,
            material.metalness,
        ));
        let fresnel = saturate(luminance(eval_fresnel(
            to_v3(specular_f0),
            to_v3(shadowed_f90(to_v3(specular_f0))),
            cgmath::dot(*v, material.shading_normal).max(0.),
        )));

        // Approximate relative contribution of BRDFs using the Fresnel term
        let specular = fresnel;
        let diffuse = diffuse_reflectance * (1.0 - fresnel); //< If diffuse term is weighted by Fresnel, apply it here as well

        // Return probability of selecting specular BRDF over diffuse BRDF
        let p = specular / (specular + diffuse).max(0.0001);

        // Clamp probability to avoid undersampling of less prominent BRDF
        clamp(p, 0.1, 0.9)
    }
}
