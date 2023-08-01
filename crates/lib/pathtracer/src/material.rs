use crate::brdf::Brdf;
use crate::math::*;

use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum Filtering {
    Nearest,
    Linear,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WrapMode {
    Clamp,
    Repeat,
    MirroredRepeat,
}

#[derive(Debug, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask(f32),
    Blend,
}

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub pixel_size: u8,
}

pub struct Sampler {
    pub filtering: Filtering,
    pub wrap_s: WrapMode,
    pub wrap_t: WrapMode,
}

impl Sampler {
    #[inline]
    fn nearest(&self, texture: &Texture, x: f32, y: f32) -> (f32, f32, f32, f32) {
        let (tx, ty) = (
            ((x + 0.5) as u32).min(texture.width - 1),
            ((y + 0.5) as u32).min(texture.height - 1),
        );

        assert!(tx < texture.width);
        assert!(ty < texture.height);

        let index = ((ty * texture.width + tx) * texture.pixel_size as u32) as usize;
        // TODO: floating point.
        (
            texture.rgba[index] as f32 / 255.,
            texture.rgba[index + 1] as f32 / 255.,
            texture.rgba[index + 2] as f32 / 255.,
            texture.rgba[index + 3] as f32 / 255.,
        )
    }

    #[inline]
    fn lerp(p1: (f32, f32, f32, f32), p2: (f32, f32, f32, f32), t: f32) -> (f32, f32, f32, f32) {
        (
            p1.0 * (1. - t) + p2.0 * t,
            p1.1 * (1. - t) + p2.1 * t,
            p1.2 * (1. - t) + p2.2 * t,
            p1.3 * (1. - t) + p2.3 * t,
        )
    }

    fn linear(&self, texture: &Texture, x: f32, y: f32) -> (f32, f32, f32, f32) {
        let sx = x.floor();
        let sy = y.floor();

        let p1 = self.nearest(texture, sx, sy);
        let p2 = self.nearest(texture, sx + 1., sy);
        let p3 = self.nearest(texture, sx, sy + 1.);
        let p4 = self.nearest(texture, sx + 1., sy + 1.);

        let x_frac = x.fract();
        let p12 = Self::lerp(p1, p2, x_frac);
        let p23 = Self::lerp(p3, p4, x_frac);

        Self::lerp(p12, p23, y.fract())
    }

    pub fn sample(&self, texture: &Texture, uv: (f32, f32)) -> (f32, f32, f32, f32) {
        // TODO: Wrap mode.
        assert_eq!(self.wrap_s, WrapMode::Repeat);
        assert_eq!(self.wrap_t, WrapMode::Repeat);

        let add = (
            if uv.0 < 0. { 1. } else { 0. },
            if uv.1 < 0. { 1. } else { 0. },
        );
        let (u, v) = (add.0 + uv.0.fract(), add.1 + uv.1.fract());

        let (x, y) = (
            u * (texture.width - 1) as f32,
            v * (texture.height - 1) as f32,
        );

        match self.filtering {
            Filtering::Linear => self.linear(texture, x, y),
            Filtering::Nearest => self.nearest(texture, x, y),
        }
    }
}

pub struct TextureSampler {
    pub texture: Arc<Texture>,
    pub sampler: Sampler,
}

impl TextureSampler {
    pub fn sample(&self, uv: (f32, f32)) -> (f32, f32, f32, f32) {
        self.sampler.sample(&self.texture, uv)
    }
}

// TODO:

pub struct Material {
    pub alpha_mode: AlphaMode,
    pub albedo_factor: Vector3,
    pub albedo_texture: Option<TextureSampler>,
    pub emitted_factor: Vector3,
    pub emitted_texture: Option<TextureSampler>,
    pub normal_texture: Option<TextureSampler>,
    pub metalic: f32,
    pub roughness: f32,
    pub metalic_roughness_texture: Option<TextureSampler>,
    pub single_sided: bool,

    pub brdf: Box<dyn Brdf + Sync + Send + 'static>,
}

impl Material {
    fn sample_texture(
        factor: &Vector3,
        texture: &Option<TextureSampler>,
        uv: (f32, f32),
    ) -> (f32, f32, f32, f32) {
        let color = match texture {
            Some(tex) => tex.sample(uv),
            None => (1., 1., 1., 1.),
        };

        (
            color.0 * factor.x,
            color.1 * factor.y,
            color.2 * factor.z,
            color.3,
        )
    }

    pub fn discard(&self, uv: (f32, f32)) -> bool {
        match self.alpha_mode {
            AlphaMode::Opaque => false,
            AlphaMode::Mask(alpha) => {
                Self::sample_texture(&self.albedo_factor, &self.albedo_texture, uv).3 <= alpha
            }
            AlphaMode::Blend => false,
        }
    }

    pub fn has_normal(&self) -> bool {
        self.normal_texture.is_some()
    }

    pub fn base_color(&self, uv: (f32, f32)) -> Vector3 {
        let color = Self::sample_texture(&self.albedo_factor, &self.albedo_texture, uv);
        Vector3::new(color.0, color.1, color.2)
    }

    pub fn emissive_color(&self, uv: (f32, f32)) -> Vector3 {
        let color = Self::sample_texture(&self.emitted_factor, &self.emitted_texture, uv);
        Vector3::new(color.0, color.1, color.2)
    }

    pub fn normal(&self, uv: (f32, f32)) -> Vector3 {
        let (x, y, z, _) = Self::sample_texture(&Vector3::one(), &self.normal_texture, uv);
        Vector3::new(x * 2. - 1., y * 2. - 1., z * 2. - 1.)
    }

    pub fn metalness(&self, uv: (f32, f32)) -> f32 {
        let (m, _, _, _) =
            Self::sample_texture(&Vector3::one(), &self.metalic_roughness_texture, uv);
        self.metalic * m
    }

    pub fn roughness(&self, uv: (f32, f32)) -> f32 {
        let (_, r, _, _) =
            Self::sample_texture(&Vector3::one(), &self.metalic_roughness_texture, uv);
        self.roughness * r
    }
}
