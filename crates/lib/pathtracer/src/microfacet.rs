use crate::brdf::ResolvedMaterial;
use crate::math::*;

use cgmath::dot;

pub struct BrdfData {
    // Material properties
    pub specular_f0: Vector3,
    pub diffuse_reflectance: Vector3,

    // Roughnesses
    pub roughness: f32,     //< perceptively linear roughness (artist's input)
    pub alpha: f32,         //< linear roughness - often 'alpha' in specular BRDF equations
    pub alpha_squared: f32, //< alpha squared - pre-calculated value commonly used in BRDF equations

    // Commonly used terms for BRDF evaluation
    pub f: Vector3, //< Fresnel term

    // Vectors
    pub v: Vector3, //< Direction to viewer (or opposite direction of incident ray)
    pub n: Vector3, //< Shading normal
    pub h: Vector3, //< Half vector (microfacet normal)
    pub l: Vector3, //< Direction to light (or direction of reflecting ray)

    pub n_dot_l: f32,
    pub n_dot_v: f32,

    pub l_dot_h: f32,
    pub n_dot_h: f32,
    pub v_dot_h: f32,

    // True when V/L is backfacing wrt. shading normal N
    pub v_backfacing: bool,
    pub l_backfacing: bool,
}

pub fn prepare_brdf_data(
    n: Vector3,
    l: Vector3,
    v: Vector3,
    material: &ResolvedMaterial,
) -> BrdfData {
    let h = (l + v).unit();

    let n_dot_l = cgmath::dot(n, l);
    let n_dot_v = cgmath::dot(n, v);

    let v_backfacing = n_dot_v <= 0.;
    let l_backfacing = n_dot_l <= 0.;

    let n_dot_l = 1.0f32.min(0.00001f32.max(n_dot_l));
    let n_dot_v = 1.0f32.min(0.00001f32.max(n_dot_v));

    let l_dot_h = saturate(cgmath::dot(l, h));
    let n_dot_h = saturate(cgmath::dot(n, h));
    let v_dot_h = saturate(cgmath::dot(v, h));

    // Unpack material properties
    let specular_f0 = base_color_to_specular_f0(material.base_color, material.metalness);
    let diffuse_reflectance =
        base_color_to_diffuse_reflectance(material.base_color, material.metalness);

    // Unpack 'perceptively linear' -> 'linear' -> 'squared' roughness
    let roughness = material.roughness;
    let alpha = material.roughness * material.roughness;
    let alpha_squared = alpha * alpha;

    // Pre-calculate some more BRDF terms
    let f = eval_fresnel(specular_f0, to_v3(shadowed_f90(specular_f0)), l_dot_h);

    BrdfData {
        specular_f0,
        diffuse_reflectance,
        roughness,
        alpha,
        alpha_squared,
        f,
        v,
        n,
        h,
        l,
        n_dot_l,
        n_dot_v,
        l_dot_h,
        n_dot_h,
        v_dot_h,
        v_backfacing,
        l_backfacing,
    }
}

// -------------------------------------------------------------------------
//    Microfacet model
// -------------------------------------------------------------------------

// Samples a microfacet normal for the GGX distribution using VNDF method.
// Source: "Sampling the GGX Distribution of Visible Normals" by Heitz
// See also https://hal.inria.fr/hal-00996995v1/document and http://jcgt.org/published/0007/04/01/
// Random variables 'u' must be in <0;1) interval
// PDF is 'G1(NdotV) * D'
pub fn sample_ggx_vndf(ve: Vector3, alpha_2d: (f32, f32), u: (f32, f32)) -> Vector3 {
    // Section 3.2: transforming the view direction to the hemisphere configuration
    let vh = Vector3::new(alpha_2d.0 * ve.x, alpha_2d.1 * ve.y, ve.z).unit();

    // Section 4.1: orthonormal basis (with special case if cross product is zero)
    let lensq = vh.x * vh.x + vh.y * vh.y;
    let tt1 = if lensq > 0. {
        Vector3::new(-vh.y, vh.x, 0.) * (1. / lensq.sqrt())
    } else {
        Vector3::new(1., 0., 0.)
    };
    let tt2 = vh.cross(tt1);

    // Section 4.2: parameterization of the projected area
    let r = u.0.sqrt();
    let phi = TWO_PI * u.1;
    let t1 = r * phi.cos();
    let mut t2 = r * phi.sin();
    let s = 0.5 * (1. + vh.z);
    t2 = lerp_scalar((1. - t1 * t1).sqrt(), t2, s);

    // Section 4.3: reprojection onto hemisphere
    let nh = vh * 0f32.max(1. - t1 * t1 - t2 * t2).sqrt() + tt1 * t1 + tt2 * t2;

    // Section 3.4: transforming the normal back to the ellipsoid configuration
    Vector3::new(alpha_2d.0 * nh.x, alpha_2d.1 * nh.y, 0f32.max(nh.z)).unit()
}

pub fn smith_g1_ggx(alpha_squared: f32, n_dot_s_squared: f32) -> f32 {
    2. / ((((alpha_squared * (1. - n_dot_s_squared)) + n_dot_s_squared).sqrt() / n_dot_s_squared)
        + 1.)
}

pub fn smith_g2_over_g1_height_correlated(
    _alpha: f32,
    alpha_squared: f32,
    n_dot_l: f32,
    n_dot_v: f32,
) -> f32 {
    let g1v = smith_g1_ggx(alpha_squared, n_dot_v * n_dot_v);
    let g1l = smith_g1_ggx(alpha_squared, n_dot_l * n_dot_l);
    g1l / (g1v + g1l - g1v * g1l)
}

// Weight for the reflection ray sampled from GGX distribution using VNDF method
pub fn specular_sample_weight_ggx_vndf(
    alpha: f32,
    alpha_squared: f32,
    n_dot_l: f32,
    n_dot_v: f32,
    _h_dot_l: f32,
    _n_dot_h: f32,
) -> f32 {
    // #if USE_HEIGHT_CORRELATED_G2
    smith_g2_over_g1_height_correlated(alpha, alpha_squared, n_dot_l, n_dot_v)
    // #else
    //     return Smith_G1_GGX(alpha, NdotL, alphaSquared, NdotL * NdotL);
    // #endif
}

// Samples a reflection ray from the rough surface using selected microfacet distribution and sampling method
// Resulting weight includes multiplication by cosine (NdotL) term
pub fn sample_specular_microfacet(
    v_local: Vector3,
    alpha: f32,
    alpha_squared: f32,
    specular_f0: Vector3,
    u: (f32, f32),
) -> (Vector3, Vector3) {
    // Sample a microfacet normal (H) in local space
    let h_local = if alpha == 0. {
        // Fast path for zero roughness (perfect reflection), also prevents NaNs appearing due to divisions by zeroes
        Vector3::new(0., 0., 1.)
    } else {
        // For non-zero roughness, this calls VNDF sampling for GG-X distribution or Walter's sampling for Beckmann distribution
        sample_ggx_vndf(v_local, (alpha, alpha), u)
    };

    // Reflect view direction to obtain light vector
    let l_local = reflect(-v_local, h_local);

    // Note: HdotL is same as HdotV here
    // Clamp dot products here to small value to prevent numerical instability. Assume that rays incident from below the hemisphere have been filtered
    let h_dot_l = 0.00001f32.max(1f32.min(dot(h_local, l_local)));
    let n_local = Vector3::new(0., 0., 1.);
    let n_dot_l = 0.00001f32.max(1f32.min(dot(n_local, l_local)));
    let n_dot_v = 0.00001f32.max(1f32.min(dot(n_local, v_local)));
    let n_dot_h = 0.00001f32.max(1f32.min(dot(n_local, h_local)));
    let f = eval_fresnel(specular_f0, to_v3(shadowed_f90(specular_f0)), h_dot_l);

    // Calculate weight of the sample specific for selected sampling method
    // (this is microfacet BRDF divided by PDF of sampling method - notice how most terms cancel out)
    let weight = f * specular_sample_weight_ggx_vndf(
        alpha,
        alpha_squared,
        n_dot_l,
        n_dot_v,
        h_dot_l,
        n_dot_h,
    );

    (l_local, weight)
}

pub fn ggx_d(alpha_squared: f32, n_dot_h: f32) -> f32 {
    let b = (alpha_squared - 1.) * n_dot_h * n_dot_h + 1.;
    alpha_squared / (std::f32::consts::PI * b * b)
}

pub fn smith_g2_height_correlated_ggx_lagarde(
    alpha_squared: f32,
    n_dot_l: f32,
    n_dot_v: f32,
) -> f32 {
    let a = n_dot_v * (alpha_squared + n_dot_l * (n_dot_l - alpha_squared * n_dot_l)).sqrt();
    let b = n_dot_l * (alpha_squared + n_dot_v * (n_dot_v - alpha_squared * n_dot_v)).sqrt();
    0.5 / (a + b)
}

pub fn eval_microfacet(data: &BrdfData) -> Vector3 {
    let d = ggx_d(0.00001f32.max(data.alpha_squared), data.n_dot_h);
    let g2 = smith_g2_height_correlated_ggx_lagarde(data.alpha_squared, data.n_dot_l, data.n_dot_v);
    //float3 F = evalFresnel(data.specularF0, shadowedF90(data.specularF0), data.VdotH); //< Unused, F is precomputed already

    // #if G2_DIVIDED_BY_DENOMINATOR
    data.f * (g2 * d * data.n_dot_l)
    // #else
    // 	return ((data.F * G2 * D) / (4.0f * data.NdotL * data.NdotV)) * data.NdotL;
    // #endif
}

pub fn eval_lambertian(data: &BrdfData) -> Vector3 {
    data.diffuse_reflectance * (ONE_OVER_PI * data.n_dot_l)
}
