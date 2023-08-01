use crate::material::Material;
use crate::math::*;

use kdtree_ray::*;
use std::{ops::Mul, sync::Arc};

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub pos: Vector3,
    pub uv: (f32, f32),
}

impl Vertex {
    pub fn new() -> Vertex {
        Vertex {
            pos: Vector3::zero(),
            uv: (0., 0.),
        }
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self::new()
    }
}

impl Mul<Vertex> for cgmath::Matrix4<f32> {
    type Output = Vertex;

    fn mul(self, rhs: Vertex) -> Self::Output {
        Vertex {
            pos: (self * rhs.pos.extend(1.0)).truncate(),
            uv: rhs.uv,
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct Triangle {
    pub vertex: [Vertex; 3],
}

impl Mul<Triangle> for cgmath::Matrix4<f32> {
    type Output = Triangle;

    fn mul(self, rhs: Triangle) -> Self::Output {
        Triangle {
            vertex: [
                self * rhs.vertex[0],
                self * rhs.vertex[1],
                self * rhs.vertex[2],
            ],
        }
    }
}

pub struct Mesh {
    pub aabb: (Vector3, Vector3),
    pub material: Arc<Material>,
    pub kd: KDtree<Triangle>,
}

impl Mesh {
    pub fn new(
        triangles: Vec<Triangle>,
        material: Arc<Material>,
        transformation: cgmath::Matrix4<f32>,
    ) -> Mesh {
        let mut aabb = (
            Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            Vector3::new(-f32::INFINITY, -f32::INFINITY, -f32::INFINITY),
        );

        let transformed: Vec<Triangle> = triangles
            .into_iter()
            .map(|tri| transformation * tri)
            .collect();

        for triangle in &transformed {
            let min = triangle.vertex[0]
                .pos
                .min(triangle.vertex[1].pos)
                .min(triangle.vertex[2].pos);
            let max = triangle.vertex[0]
                .pos
                .max(triangle.vertex[1].pos)
                .max(triangle.vertex[2].pos);

            aabb = (aabb.0.min(min), aabb.1.max(max));
        }

        Mesh {
            aabb,
            material,
            kd: KDtree::new(transformed),
        }
    }
}

// To use the KDtree on an object you need first to implement the BoundingBox trait.
impl BoundingBox for Triangle {
    fn bounding_box(&self) -> AABB {
        let min = cgmath::Vector3::new(
            self.vertex[0]
                .pos
                .x
                .min(self.vertex[1].pos.x)
                .min(self.vertex[2].pos.x),
            self.vertex[0]
                .pos
                .y
                .min(self.vertex[1].pos.y)
                .min(self.vertex[2].pos.y),
            self.vertex[0]
                .pos
                .z
                .min(self.vertex[1].pos.z)
                .min(self.vertex[2].pos.z),
        );
        let max = cgmath::Vector3::new(
            self.vertex[0]
                .pos
                .x
                .max(self.vertex[1].pos.x)
                .max(self.vertex[2].pos.x),
            self.vertex[0]
                .pos
                .y
                .max(self.vertex[1].pos.y)
                .max(self.vertex[2].pos.y),
            self.vertex[0]
                .pos
                .z
                .max(self.vertex[1].pos.z)
                .max(self.vertex[2].pos.z),
        );
        [min, max]
    }
}

impl BoundingBox for Mesh {
    fn bounding_box(&self) -> AABB {
        [self.aabb.0, self.aabb.1]
    }
}
