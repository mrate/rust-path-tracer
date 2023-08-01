use super::renderable::{Renderable, Vertex};

use pathtracer::math::Vector3;
use pathtracer::scene::SceneImportHandler;

fn convert_vertices(input: &[f32]) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(input.len());
    for v in input.chunks(8) {
        out.push(Vertex {
            position: [v[0], v[1], v[2]],
            uv: [v[3], v[4]],
            normal: [v[5], v[6], v[7]],
        });
    }
    out
}

fn to_vec(color: Vector3) -> Vec<u8> {
    vec![
        (color.x * 255.) as u8,
        (color.y * 255.) as u8,
        (color.z * 255.) as u8,
        255u8,
    ]
}

struct TextureInfo {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

struct MeshInfo {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

pub struct ImportHandler {
    //display: &'a glium::Display,
    // pub renderables: Vec<renderable::Renderable>,
    // pub textures: Vec<glium::texture::Texture2d>,
    meshes: Vec<MeshInfo>,
    textures: Vec<TextureInfo>,
    texture_mapping: Vec<i32>,
    // vs: &'a str,
    // fs: &'a str,
}

impl ImportHandler {
    pub fn new() -> Self {
        Self {
            meshes: vec![],
            textures: vec![],
            texture_mapping: vec![],
        }
    }

    pub fn generate(
        self,
        display: &glium::Display,
        vs: &str,
        fs: &str,
    ) -> (Vec<Renderable>, Vec<glium::texture::Texture2d>, Vec<i32>) {
        let meshes = self
            .meshes
            .into_iter()
            .map(|mesh| Renderable::new(display, vs, fs, &mesh.vertices, &mesh.indices, false))
            .collect();

        let textures = self
            .textures
            .into_iter()
            .map(
                |TextureInfo {
                     width,
                     height,
                     data,
                 }| {
                    let texture_data =
                        glium::texture::RawImage2d::from_raw_rgba(data, (width, height));

                    let format = glium::texture::UncompressedFloatFormat::U8U8U8U8;
                    let mips = glium::texture::MipmapsOption::AutoGeneratedMipmaps;

                    glium::Texture2d::with_format(display, texture_data, format, mips).unwrap()
                },
            )
            .collect();

        (meshes, textures, self.texture_mapping)
    }
}

impl SceneImportHandler for ImportHandler {
    fn handle_material(&mut self, color: Vector3, texture: Option<(u32, u32, &[u8])>) {
        // TODO: Material without textures.

        self.textures.push(match texture {
            Some((w, h, data)) => TextureInfo {
                width: w,
                height: h,
                data: data.to_vec(),
            },
            None => TextureInfo {
                width: 1,
                height: 1,
                data: to_vec(color),
            },
        });
    }

    fn handle_mesh(&mut self, vertices: &[f32], indices: &[u32], material_index: i32) {
        // TODO: Material without textures.

        self.meshes.push(MeshInfo {
            vertices: convert_vertices(vertices),
            indices: indices.to_vec(),
        });

        self.texture_mapping.push(material_index);
    }

    fn handle_ortho_camera(&mut self, width: f32, height: f32, near: f32, far: f32) {
        println!(
            "TODO: ortographic camera: width={width}, height={height}, near={near}, far={far}"
        );
    }

    fn handle_perspective_camera(&mut self, v_fov: f32, aspect_ratio: f32, near: f32, far: f32) {
        println!(
            "TODO: perspective camera: v_fov={v_fov}, aspect={aspect_ratio}, near={near}, far={far}"
        );
    }

    fn handle_camera_transform(
        &mut self,
        camera_index: usize,
        translate: &[f32; 3],
        rotate: &[f32; 4],
    ) {
        println!(
            "Camera transform[{camera_index}] = {:?};{:?}",
            translate, rotate
        );
    }
}
