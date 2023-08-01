use cgmath::One;
use serde::*;

use std::fs::File;
use std::path::Path;

use crate::env;
use crate::light::{Directional, Light, Point};
use crate::math::*;
use crate::Error;

#[derive(Serialize, Deserialize)]
pub struct TransformationDescription {
    pub translate: Option<(f32, f32, f32)>,
    pub scale: Option<(f32, f32, f32)>,
    pub rotate: Option<(f32, f32, f32)>,
}

impl TransformationDescription {
    pub fn to_matrix(&self) -> cgmath::Matrix4<f32> {
        let mut result = cgmath::Matrix4::one();

        if let Some(t) = self.translate {
            result = result * cgmath::Matrix4::from_translation(Vector3::new(t.0, t.1, t.2));
        }
        if let Some(s) = self.scale {
            result = result * cgmath::Matrix4::from_nonuniform_scale(s.0, s.1, s.2);
        }
        if let Some(r) = self.rotate {
            result = result
                * cgmath::Matrix4::from_angle_x(cgmath::Deg(r.0))
                * cgmath::Matrix4::from_angle_y(cgmath::Deg(r.1))
                * cgmath::Matrix4::from_angle_z(cgmath::Deg(r.2));
        }

        result
    }
}

#[derive(Serialize, Deserialize)]
pub struct MeshDescription {
    name: String,
    path: String,
    transformation: Option<TransformationDescription>,
}

impl MeshDescription {
    pub fn transformation(&self) -> cgmath::Matrix4<f32> {
        self.transformation
            .as_ref()
            .map_or(cgmath::Matrix4::one(), |t| t.to_matrix())
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Serialize, Deserialize)]
struct DirLightDescription {
    dir: (f32, f32, f32),
    color: (f32, f32, f32),
    intensity: f32,
}

impl DirLightDescription {
    fn to_light(&self) -> Light {
        Light::Directional(Directional {
            dir: Vector3::new(self.dir.0, self.dir.1, self.dir.2),
            color: Vector3::new(self.color.0, self.color.1, self.color.2),
            intensity: self.intensity,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct PointLightDescription {
    position: (f32, f32, f32),
    color: (f32, f32, f32),
    intensity: f32,
    range: f32,
}

impl PointLightDescription {
    fn to_light(&self) -> Light {
        Light::Point(Point {
            position: Vector3::new(self.position.0, self.position.1, self.position.2),
            color: Vector3::new(self.color.0, self.color.1, self.color.2),
            intensity: self.intensity,
            range: self.range,
            range_squared: self.range * self.range,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub enum EnvironmentDescription {
    Black,
    Gradient((f32, f32, f32), (f32, f32, f32)),
}

#[derive(Serialize, Deserialize)]
pub struct SceneDescription {
    meshes: Vec<MeshDescription>,
    dir_lights: Option<Vec<DirLightDescription>>,
    point_lights: Option<Vec<PointLightDescription>>,
    env: Option<EnvironmentDescription>,
}

impl SceneDescription {
    pub fn from_file(filename: &Path) -> Result<Self, Error> {
        let file = File::open(filename)?;
        let description: SceneDescription = serde_json::from_reader(file)?;
        Ok(description)
    }

    pub fn meshes(&self) -> &Vec<MeshDescription> {
        &self.meshes
    }

    pub fn lights(self) -> Vec<Light> {
        self.dir_lights
            .unwrap_or_default()
            .into_iter()
            .map(|desc| desc.to_light())
            .chain(
                self.point_lights
                    .unwrap_or_default()
                    .into_iter()
                    .map(|desc| desc.to_light()),
            )
            .collect()
    }

    pub fn environment(&self) -> Box<dyn env::Environment + Sync + Send> {
        match &self.env {
            None => Box::new(env::Black {}),
            Some(env) => match env {
                EnvironmentDescription::Black => Box::new(env::Black {}),
                EnvironmentDescription::Gradient(from, to) => Box::new(env::Gradient::new(
                    Vector3::new(from.0, from.1, from.2),
                    Vector3::new(to.0, to.1, to.2),
                )),
            },
        }
    }
}
