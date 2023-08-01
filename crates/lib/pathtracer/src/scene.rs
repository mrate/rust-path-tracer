use crate::brdf::Hit;
use crate::env;
use crate::import_scene::*;
use crate::light::Light;
use crate::math::*;
use crate::mesh::*;
use crate::ray::{self, ray_triangle_intersection};
use crate::{import_gltf, Error};

use kdtree_ray::*;

use std::path::Path;

pub struct Scene {
    kd: KDtree<Mesh>,

    lights: Vec<Light>,
    spheres: Vec<Sphere>,

    env: Box<dyn env::Environment + Send + Sync>,
}

pub trait SceneImportHandler {
    fn handle_material(&mut self, color: Vector3, texture: Option<(u32, u32, &[u8])>);
    fn handle_mesh(&mut self, vertices: &[f32], indices: &[u32], material_index: i32);
    fn handle_ortho_camera(&mut self, width: f32, height: f32, near: f32, far: f32);
    fn handle_perspective_camera(&mut self, v_fov: f32, aspect_ratio: f32, near: f32, far: f32);
    fn handle_camera_transform(
        &mut self,
        camera_index: usize,
        translate: &[f32; 3],
        rotate: &[f32; 4],
    );
}

impl Scene {
    pub fn empty() -> Scene {
        Scene {
            kd: KDtree::new(vec![]),
            lights: vec![],
            spheres: vec![],
            env: Box::new(env::Black {}),
        }
    }

    pub fn load<H>(filename: &Path, handler: &mut Option<&mut H>) -> Result<Scene, Error>
    where
        H: SceneImportHandler,
    {
        let description = SceneDescription::from_file(filename)?;

        let mut meshes = vec![];
        for mesh in description.meshes() {
            meshes.append(&mut import_gltf::load(
                Path::new(mesh.path()),
                mesh.transformation(),
                handler,
            )?);
        }

        let kd = KDtree::new(meshes);

        // self.spheres = vec![
        //     Sphere::position_radius(0.0, 0.0, -1.0, 0.5, sphere_material.clone()),
        //     Sphere::position_radius(0.0, 100.5, -1.0, 100.0, sphere_material.clone()),
        // ];

        let env = description.environment();

        Ok(Scene {
            kd,
            lights: description.lights(),
            spheres: vec![],
            env,
        })
    }

    pub fn environment(&self, ray: &ray::Ray) -> Vector3 {
        self.env.color(ray)
    }

    fn hit_triangles(&self, ray: &ray::Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        optick::event!("hit tris");

        let mut result: Option<Hit> = None;

        let ray_origin = cgmath::Vector3::new(ray.origin.x, ray.origin.y, ray.origin.z);
        let ray_direction = cgmath::Vector3::new(ray.direction.x, ray.direction.y, ray.direction.z);

        let meshes = {
            optick::event!("kd_mesh");
            self.kd.intersect(&ray_origin, &ray_direction)
        };
        optick::tag!("mesh_cnt", meshes.len() as i32);

        for mesh in &meshes {
            let triangles = {
                optick::event!("kd_tris");
                mesh.kd.intersect(&ray_origin, &ray_direction)
            };
            optick::tag!("tris_cnt", triangles.len() as i32);

            for triangle in &triangles {
                let intersection = ray_triangle_intersection(
                    ray,
                    triangle,
                    mesh.material.single_sided,
                    t_min,
                    t_max,
                );

                let prev_distance = match &result {
                    Some(hit) => hit.t,
                    None => f32::INFINITY,
                };

                result = match intersection {
                    Some(tr_int) if prev_distance > tr_int.t => {
                        let point = ray.point_at(tr_int.t);
                        Some(Hit {
                            position: point,
                            material: mesh.material.clone(),
                            t: tr_int.t,
                            uv: tr_int.uv,
                            normal: tr_int.normal,
                            tangent: tr_int.tangent,
                            bitangent: tr_int.bitangent,
                        })
                    }
                    Some(_) => result,
                    None => result,
                }
            }
        }

        result
    }

    pub fn hit_spheres(&self, ray: &ray::Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        let mut result: Option<Hit> = None;

        for sphere in &self.spheres {
            let intersection = ray::ray_sphere_intersection(ray, sphere, t_min, t_max);

            let prev_distance = match &result {
                Some(hit) => hit.t,
                None => f32::INFINITY,
            };

            result = match intersection {
                Some(distance) if prev_distance > distance => {
                    let point = ray.point_at(distance);
                    let uv = (0.0, 0.0); // TODO:

                    Some(Hit {
                        position: point,
                        material: sphere.material.clone(),
                        t: distance,
                        uv,
                        normal: sphere.normal(point),
                        tangent: Vector3::zero(),
                        bitangent: Vector3::zero(),
                    })
                }
                Some(_) => result,
                None => result,
            }
        }

        result
    }

    pub fn hit(&self, ray: &ray::Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        let mut current_ray = *ray;
        let mut current_t_max = t_max;

        loop {
            match self.hit_triangles(&current_ray, t_min, current_t_max) {
                Some(hit) if hit.material.discard(hit.uv) => {
                    current_ray = ray::Ray::new(hit.position, ray.direction);
                    current_t_max -= hit.t;
                }
                Some(hit) => return Some(hit),
                None => {
                    return None;
                }
            }
        }
        //self.hit_spheres(ray, t_min, t_max)
    }

    pub fn lights(&self) -> &Vec<Light> {
        &self.lights
    }
}
