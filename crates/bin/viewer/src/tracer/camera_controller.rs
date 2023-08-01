use ::pathtracer::*;

use ::cgmath::*;

use serde::{Deserialize, Serialize};

pub struct CameraController {
    pub yaw: f32,
    pub pitch: f32,
    position: Vector3<f32>,
    up: Vector3<f32>,
    z_near: f32,
    z_far: f32,
    aspect_ratio: f32,
    pub v_fov: f32,
    pub aperture: f32,
    pub focus_distance: f32,
    pub simple_camera: bool,
    pub speed: f32,
}

#[derive(Debug, Deserialize, Serialize)]
struct CameraStore {
    position: (f32, f32, f32),
    yaw: f32,
    pitch: f32,
    v_fov: f32,
    aperture: f32,
    focus_distance: f32,
    simple_camera: bool,
    speed: Option<f32>,
}

impl CameraController {
    pub fn new(z_near: f32, z_far: f32, aspect_ratio: f32, v_fov: f32) -> CameraController {
        CameraController {
            yaw: 0.,
            pitch: 0.,
            position: Vector3::new(0., 0., 0.),
            up: Vector3::new(0., 1., 0.),
            z_near,
            z_far,
            aspect_ratio,
            v_fov,
            aperture: 1.,
            focus_distance: 1.,
            simple_camera: true,
            speed: 10.,
        }
    }

    pub fn lookat(&self) -> Vector3<f32> {
        self.position + self.forward()
    }

    pub fn move_offset(&mut self, dir: &Vector3<f32>) {
        self.position += dir * self.speed;
    }

    pub fn tracer_camera(&self) -> Box<dyn camera::Camera + Send + Sync> {
        match self.simple_camera {
            true => self.tracer_simple_camera(),
            false => self.tracer_focus_camera(),
        }
    }

    pub fn tracer_simple_camera(&self) -> Box<dyn camera::Camera + Send + Sync> {
        let pos = self.position;
        let look_at = self.lookat();

        Box::new(camera::SimpleCamera::look_at(
            math::Vector3::new(pos.x, pos.y, pos.z),
            math::Vector3::new(look_at.x, look_at.y, look_at.z),
            math::Vector3::new(0., 1., 0.),
            self.v_fov,
            self.aspect_ratio,
        ))
    }

    pub fn tracer_focus_camera(&self) -> Box<dyn camera::Camera + Send + Sync> {
        let pos = self.position;
        let look_at = self.lookat();

        Box::new(camera::ApertureCamera::look_at(
            math::Vector3::new(pos.x, pos.y, pos.z),
            math::Vector3::new(look_at.x, look_at.y, look_at.z),
            math::Vector3::new(0., 1., 0.),
            self.v_fov,
            self.aspect_ratio,
            self.aperture,
            self.focus_distance,
        ))
    }

    pub fn gl_camera(&self) -> [[f32; 4]; 4] {
        let pos = self.position;
        let look_at = self.lookat();

        let view_matrix = Matrix4::look_at_rh(
            Point3::new(pos.x, pos.y, pos.z),
            Point3::new(look_at.x, look_at.y, look_at.z),
            self.up,
        );

        let projection_matrix =
            perspective(Deg(self.v_fov), self.aspect_ratio, self.z_near, self.z_far);

        (projection_matrix * view_matrix).into()
    }

    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    pub fn direction(&self) -> Quaternion<f32> {
        Quaternion::from(Euler::new(Rad(0.), Rad(self.yaw), Rad(0.)))
            * Quaternion::from(Euler::new(Rad(self.pitch), Rad(0.), Rad(0.)))
    }

    pub fn forward(&self) -> Vector3<f32> {
        self.direction() * Vector3::new(0., 0., 1.)
    }

    pub fn right(&self) -> Vector3<f32> {
        self.direction() * Vector3::new(1., 0., 0.)
    }

    pub fn up(&self) -> Vector3<f32> {
        self.up
    }

    pub fn reset(&mut self) {
        self.position = Vector3::zero();
        self.simple_camera = true;
        self.yaw = 0.;
        self.pitch = 0.;
    }

    pub fn save(&self, file: &str) -> Result<(), std::io::Error> {
        let store = CameraStore {
            position: (self.position.x, self.position.y, self.position.z),
            yaw: self.yaw,
            pitch: self.pitch,
            v_fov: self.v_fov,
            aperture: self.aperture,
            focus_distance: self.focus_distance,
            simple_camera: self.simple_camera,
            speed: Some(self.speed),
        };

        let file = std::fs::File::create(file)?;
        serde_json::to_writer(file, &store).map_err(std::io::Error::from)
    }

    pub fn load(&mut self, file: &str) -> Result<(), std::io::Error> {
        let file = std::fs::File::open(file)?;
        let store = serde_json::from_reader::<_, CameraStore>(file);

        match store {
            Ok(store) => {
                self.position = Vector3::new(store.position.0, store.position.1, store.position.2);
                self.yaw = store.yaw;
                self.pitch = store.pitch;
                self.v_fov = store.v_fov;
                self.aperture = store.aperture;
                self.focus_distance = store.focus_distance;
                self.simple_camera = store.simple_camera;
                self.speed = store.speed.unwrap_or(10.);
                Ok(())
            }
            Err(err) => Err(std::io::Error::from(err)),
        }
    }
}
