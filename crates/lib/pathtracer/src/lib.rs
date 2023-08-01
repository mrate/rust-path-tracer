pub mod camera;
pub mod material;
pub mod math;
pub mod pathtracer;
pub mod random;
pub mod scene;
pub mod threadpool;

mod brdf;
mod brdf_lambert;
mod brdf_microfacet;
mod consts;
mod env;
mod import_gltf;
mod import_scene;
mod light;
mod mesh;
mod microfacet;
mod ray;

#[derive(Debug)]
pub enum Error {
    ImportError(String),
    IoError(String),
    FormatError(String),
}

impl From<gltf::Error> for Error {
    fn from(e: gltf::Error) -> Self {
        Error::ImportError(format!("{:?}", e))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(format!("{:?}", e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::FormatError(format!("{:?}", e))
    }
}
