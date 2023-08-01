use rand::distributions::Uniform;
use rand::{thread_rng, Rng};

use crate::math::{EnhancedVector, Vector3};

pub trait Sampler {
    fn next_float(&self) -> f32;
    fn next_float_norm(&self) -> f32;
}

pub fn unit_sphere(sampler: &impl Sampler) -> Vector3 {
    loop {
        let vector = Vector3::new(
            sampler.next_float_norm(),
            sampler.next_float_norm(),
            sampler.next_float_norm(),
        );
        if vector.squared_length() < 1. {
            return vector;
        }
    }
}

pub fn unit_disk(sampler: &impl Sampler) -> Vector3 {
    loop {
        let p = Vector3::new(sampler.next_float_norm(), sampler.next_float_norm(), 0.);
        if p.squared_length() < 1. {
            return p;
        }
    }
}

pub struct UniformSampler {
    sampler: Uniform<f32>,
}

impl UniformSampler {
    pub fn new() -> Self {
        Self {
            sampler: Uniform::new(-1., 1.),
        }
    }
}

impl Default for UniformSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler for UniformSampler {
    fn next_float_norm(&self) -> f32 {
        thread_rng().sample(self.sampler)
    }

    fn next_float(&self) -> f32 {
        (thread_rng().sample(self.sampler) + 1.) / 2.
    }
}

pub struct ZeroSampler();

impl Sampler for ZeroSampler {
    fn next_float_norm(&self) -> f32 {
        0.
    }

    fn next_float(&self) -> f32 {
        0.
    }
}
