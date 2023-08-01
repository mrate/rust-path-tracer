use serde::Deserialize;
use serde::Serialize;
use serde_json;

use crate::brdf::*;
use crate::camera;
use crate::light::Attenuable;
use crate::light::Light;
use crate::math::*;
use crate::random::{Sampler, UniformSampler};
use crate::ray::Ray;
use crate::scene;
use crate::Error;

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct TracerSettings {
    pub max_scatter_depth: u32,
    pub shadow_rays: bool,
    pub random_light_sample: bool,
    pub t_min: f32,
    pub t_max: f32,
    pub min_bounces: u32,
}

impl TracerSettings {
    pub fn from_file(path: &str) -> Result<Self, Error> {
        let mut settings = TracerSettings::default();
        settings.load(path).map(|_| settings)
    }

    pub fn save(&self, path: &str) -> Result<(), Error> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer(file, &self).map_err(Error::from)
    }

    pub fn load(&mut self, path: &str) -> Result<(), Error> {
        let file = std::fs::File::open(path)?;
        *self = serde_json::from_reader(file)?;
        Ok(())
    }
}

pub struct Tracer {
    settings: TracerSettings,
    camera: Box<dyn camera::Camera + Send + Sync>,
    scene: scene::Scene,
}

impl Drop for Tracer {
    fn drop(&mut self) {
        // optick::stop_capture("tracer");
    }
}

impl Tracer {
    pub fn new(
        camera: Box<dyn camera::Camera + Send + Sync>,
        scene: scene::Scene,
        settings: TracerSettings,
    ) -> Tracer {
        // optick::start_capture();
        Tracer {
            settings,
            camera,
            scene,
        }
    }

    pub fn scene(&self) -> &scene::Scene {
        &self.scene
    }

    pub fn set_scene(&mut self, scene: scene::Scene) {
        self.scene = scene
    }

    pub fn set_camera(&mut self, camera: Box<dyn camera::Camera + Send + Sync>) {
        self.camera = camera;
    }

    pub fn set_settings(&mut self, settings: TracerSettings) {
        self.settings = settings;
    }

    /// Returns direction to light from given position if the light is visible from that position.
    fn trace_light(&self, position: &Vector3, normal: &Vector3, light: &Light) -> Option<Vector3> {
        // Shortcut for point lights too far away.
        if let Light::Point(point) = light {
            if (point.position - *position).squared_length() > point.range_squared {
                return None;
            }
        }

        let (direction, distance) = light.direction_distance_from(position);

        if cgmath::dot(*normal, direction) <= 0. {
            return None;
        }

        // TODO: Hitting dielectric material...
        let hit = self.scene.hit(
            &Ray::new(*position, direction),
            self.settings.t_min,
            self.settings.t_max,
        );

        match hit {
            Some(hit) if hit.t >= distance => Some(direction),
            None => Some(direction),
            _ => None,
        }
    }

    fn sample_light(
        &self,
        light: &Light,
        position: &Vector3,
        material: &ResolvedMaterial,
        brdf: &(dyn Brdf + Send + Sync),
        wo: &Vector3,
    ) -> Vector3 {
        match self.trace_light(position, &material.shading_normal, light) {
            Some(light_dir) => {
                brdf.eval(&light_dir, wo, material)
                    .mul(light.intensity_at(position))
                    / brdf.pdf(&light_dir, &material.shading_normal)
            }
            _ => Vector3::zero(),
        }
    }

    fn sample_lights(
        &self,
        position: &Vector3,
        material: &ResolvedMaterial,
        brdf: &(dyn Brdf + Send + Sync),
        sampler: &UniformSampler,
        wo: &Vector3,
    ) -> Vector3 {
        // https://computergraphics.stackexchange.com/questions/5152/progressive-path-tracing-with-explicit-light-sampling
        // For each light:
        optick::event!("lights");

        let num_lights = self.scene.lights().len();
        if num_lights == 0 {
            return Vector3::zero();
        }

        if self.settings.random_light_sample {
            let random_light = (sampler.next_float() * num_lights as f32) as usize;
            let light = &self.scene.lights()[random_light];

            self.sample_light(light, position, material, brdf, wo) / (num_lights as f32)
        } else {
            let mut color = Vector3::zero();

            for light in self.scene.lights() {
                color += self.sample_light(light, position, material, brdf, wo);
            }

            color / (num_lights as f32)
        }
    }

    pub fn trace(&self, x: f32, y: f32) -> Vector3 {
        optick::event!("trace");
        let sampler = UniformSampler::new();

        let mut ray = self.camera.ray(x, y, &sampler);
        let mut color = Vector3::zero();
        let mut throughput = Vector3::one();
        let mut bounce = 0;

        while bounce < self.settings.max_scatter_depth {
            optick::event!("bounce");

            bounce += 1;

            // No hit.
            let hit = match self
                .scene
                .hit(&ray, self.settings.t_min, self.settings.t_max)
            {
                None => {
                    color += throughput.mul(self.scene.environment(&ray));
                    break;
                }
                Some(hit) => hit,
            };

            let material = hit.resolve_material();
            let brdf = &(*hit.material.brdf);
            let v = -ray.direction;

            color += throughput.mul(material.emissive);

            // Direct light sampling.
            if self.settings.shadow_rays {
                color += throughput.mul(self.sample_lights(
                    &hit.position,
                    &material,
                    brdf,
                    &sampler,
                    &ray.direction,
                ));
            }

            if bounce == self.settings.max_scatter_depth {
                break;
            }

            // Russian rulette: https://pbr-book.org/3ed-2018/Monte_Carlo_Integration/Russian_Roulette_and_Splitting
            if bounce > self.settings.min_bounces {
                let prob = luminance(throughput).min(0.95);
                if prob < sampler.next_float() {
                    break;
                }
                throughput /= prob; // TODO: why?
            }

            let brdf_type = if material.metalness == 1. && material.roughness == 0. {
                BrdfType::Specular
            } else {
                let prob = brdf.probability(&v, &material);

                if sampler.next_float() < prob {
                    throughput /= prob;
                    BrdfType::Specular
                } else {
                    throughput /= 1. - prob;
                    BrdfType::Diffuse
                }
            };

            let wi = match brdf.sample(brdf_type, &ray.direction, &material, &sampler) {
                Some(wi) => wi,
                None => break,
            };

            let pdf = brdf.pdf(&wi, &material.shading_normal);

            // Eval.
            let mat_color = brdf.eval(&wi, &ray.direction, &material);
            throughput = throughput.mul(mat_color) / pdf;

            ray = Ray::new(hit.position, wi);
        }

        match self.settings.max_scatter_depth {
            1 => color + throughput, // Special case for single bounce.
            _ => color,
        }
    }
}
