use std::f32::EPSILON;

use crate::math::*;
use crate::mesh::*;

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
}

impl Ray {
    pub fn new(origin: Vector3, direction: Vector3) -> Self {
        Self { origin, direction }
    }

    pub fn point_at(&self, distance: f32) -> Vector3 {
        self.origin + self.direction * distance
    }
}

type Intersection = Option<f32>;

pub fn ray_sphere_intersection(ray: &Ray, sphere: &Sphere, t_min: f32, t_max: f32) -> Intersection {
    let ray_to_center = ray.origin - sphere.center;

    let a = cgmath::dot(ray.direction, ray.direction);
    let b = 2.0 * cgmath::dot(ray_to_center, ray.direction);
    let c = cgmath::dot(ray_to_center, ray_to_center) - sphere.radius * sphere.radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant > 0.0 {
        let discriminant_sq = discriminant.sqrt();

        let xt = (-b - discriminant_sq) / (2.0 * a);
        if xt >= t_min && xt <= t_max {
            return Some(xt);
        }

        let xt = (-b + discriminant_sq) / (2.0 * a);
        if xt >= t_min && xt <= t_max {
            return Some(xt);
        }
    }

    None
}

pub struct TriangleIntersection {
    pub t: f32,
    pub uv: (f32, f32),
    pub normal: Vector3,
    pub tangent: Vector3,
    pub bitangent: Vector3,
}

pub fn ray_triangle_intersection(
    ray: &Ray,
    triangle: &Triangle,
    single_sided: bool,
    t_min: f32,
    t_max: f32,
) -> Option<TriangleIntersection> {
    //optick::event!("ray_triangle_intersection");

    let v0v1 = triangle.vertex[1].pos - triangle.vertex[0].pos;
    let v0v2 = triangle.vertex[2].pos - triangle.vertex[0].pos;

    let pvec = ray.direction.cross(v0v2);
    let det = cgmath::dot(v0v1, pvec);

    // ray and triangle are parallel if det is close to 0
    if det.abs() < EPSILON {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = ray.origin - triangle.vertex[0].pos;

    let u = cgmath::dot(tvec, pvec) * inv_det;
    if !(0. ..=1.).contains(&u) {
        return None;
    }

    let qvec = tvec.cross(v0v1);
    let v = cgmath::dot(ray.direction, qvec) * inv_det;
    if v < 0. || u + v > 1. {
        return None;
    }

    // Backface culling - counter clock-wise.
    let normal = v0v1.cross(v0v2).unit();
    if cgmath::dot(ray.direction, normal) > 0. && single_sided {
        return None;
    }

    let t = cgmath::dot(v0v2, qvec) * inv_det;
    if t < t_min || t > t_max {
        return None;
    }

    // https://learnopengl.com/Advanced-Lighting/Normal-Mapping
    let uv0v1_x = triangle.vertex[1].uv.0 - triangle.vertex[0].uv.0;
    let uv0v1_y = triangle.vertex[1].uv.1 - triangle.vertex[0].uv.1;
    let uv0v2_x = triangle.vertex[2].uv.0 - triangle.vertex[0].uv.0;
    let uv0v2_y = triangle.vertex[2].uv.1 - triangle.vertex[0].uv.1;

    let bu = triangle.vertex[0].uv.0 + u * uv0v1_x + v * uv0v2_x;
    let bv = triangle.vertex[0].uv.1 + u * uv0v1_y + v * uv0v2_y;

    // TODO: Make optional.
    let f = 1. / (uv0v1_x * uv0v2_y - uv0v2_x * uv0v1_y);

    let tangent = (f * (uv0v2_y * v0v1 - uv0v1_y * v0v2)).unit();
    let bitangent = (f * (-uv0v2_x * v0v1 + uv0v1_x * v0v2)).unit();

    Some(TriangleIntersection {
        t,
        uv: (bu, bv),
        normal,
        tangent,
        bitangent,
    })
}
