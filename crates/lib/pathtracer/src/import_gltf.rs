use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::brdf_lambert::{Dielectric, Lambertian};
use crate::brdf_microfacet::MicrofacetBrdf;

use crate::material::{AlphaMode, Filtering, Material, Sampler, Texture, TextureSampler, WrapMode};
use crate::math::{EnhancedVector, Vector3};
use crate::mesh::{Mesh, Triangle, Vertex};
use crate::scene::SceneImportHandler;
use crate::Error;

fn handle_material<H>(handler: &mut H, material: &Material)
where
    H: SceneImportHandler,
{
    let albedo = material.albedo_texture.as_ref().map(|texture| {
        (
            texture.texture.width,
            texture.texture.height,
            &texture.texture.rgba[..],
        )
    });

    handler.handle_material(material.albedo_factor, albedo);
}

fn handle_mesh<H>(
    handler: &mut H,
    material_index: i32,
    transformation: cgmath::Matrix4<f32>,
    vertices: &[[f32; 3]],
    coords: &[[f32; 2]],
    normals: &[[f32; 3]],
    indices: &[usize],
) where
    H: SceneImportHandler,
{
    let mut out_vertices = Vec::with_capacity(vertices.len() * 8);

    for (i, vertex) in vertices.iter().enumerate() {
        let t_vertex =
            (transformation * Vector3::new(vertex[0], vertex[1], vertex[2]).extend(1.0)).truncate();

        let coord = if coords.is_empty() {
            [0.; 2]
        } else {
            coords[i]
        };

        let normal = if normals.is_empty() {
            [0.; 3]
        } else {
            normals[i]
        };

        out_vertices.push(t_vertex.x);
        out_vertices.push(t_vertex.y);
        out_vertices.push(t_vertex.z);
        out_vertices.push(coord[0]);
        out_vertices.push(coord[1]);
        out_vertices.push(normal[0]);
        out_vertices.push(normal[1]);
        out_vertices.push(normal[2]);
    }

    let out_indices: Vec<u32> = indices.iter().map(|i| *i as u32).collect();

    handler.handle_mesh(&out_vertices, &out_indices, material_index);
}

fn to_sampler(sampler: &gltf::texture::Sampler) -> Sampler {
    let filtering = match sampler.mag_filter() {
        Some(gltf::texture::MagFilter::Linear) => Filtering::Linear,
        _ => Filtering::Nearest,
    };

    let wrap_s = match sampler.wrap_s() {
        gltf::texture::WrappingMode::ClampToEdge => WrapMode::Clamp,
        gltf::texture::WrappingMode::MirroredRepeat => WrapMode::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => WrapMode::Repeat,
    };

    let wrap_t = match sampler.wrap_t() {
        gltf::texture::WrappingMode::ClampToEdge => WrapMode::Clamp,
        gltf::texture::WrappingMode::MirroredRepeat => WrapMode::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => WrapMode::Repeat,
    };

    Sampler {
        filtering,
        wrap_s,
        wrap_t,
    }
}

#[inline]
fn from_u16(lsb: u8, msb: u8) -> u8 {
    (255. * (((lsb as u16) | ((msb as u16) << 8)) as f32 / 165535.)) as u8
}

fn to_texture(texture: &gltf::image::Data) -> Texture {
    use gltf::image::Format;

    let reader = match texture.format {
        Format::R8 => |data: &Vec<u8>, i| {
            let rgba = (data[i], 0, 0, 255);
            (1, rgba)
        },
        Format::R8G8 => |data: &Vec<u8>, i| {
            let rgba = (data[i], data[i + 1], 0, 255);
            (2, rgba)
        },
        Format::R8G8B8 => |data: &Vec<u8>, i| {
            let rgba = (data[i], data[i + 1], data[i + 2], 255);
            (3, rgba)
        },
        Format::R8G8B8A8 => |data: &Vec<u8>, i| {
            let rgba = (data[i], data[i + 1], data[i + 2], data[i + 3]);
            (4, rgba)
        },
        Format::B8G8R8 => |data: &Vec<u8>, i| {
            let rgba = (data[i + 2], data[i + 1], data[i], 255);
            (3, rgba)
        },
        Format::B8G8R8A8 => |data: &Vec<u8>, i| {
            let rgba = (data[i + 2], data[i + 1], data[i], data[i + 3]);
            (4, rgba)
        },
        Format::R16 => |data: &Vec<u8>, i| {
            let rgba = (from_u16(data[i], data[i + 1]), 0u8, 0u8, 255u8);
            (2, rgba)
        },
        Format::R16G16 => |data: &Vec<u8>, i| {
            let rgba = (
                from_u16(data[i], data[i + 1]),
                from_u16(data[i + 2], data[i + 3]),
                0u8,
                255u8,
            );
            (4, rgba)
        },
        Format::R16G16B16 => |data: &Vec<u8>, i| {
            let rgba = (
                from_u16(data[i], data[i + 1]),
                from_u16(data[i + 2], data[i + 3]),
                from_u16(data[i + 4], data[i + 5]),
                255u8,
            );
            (6, rgba)
        },
        Format::R16G16B16A16 => |data: &Vec<u8>, i| {
            let rgba = (
                from_u16(data[i], data[i + 1]),
                from_u16(data[i + 2], data[i + 3]),
                from_u16(data[i + 4], data[i + 5]),
                from_u16(data[i + 6], data[i + 7]),
            );
            (8, rgba)
        },
    };

    let pixel_size = 4u8;
    let mut rgba: Vec<u8> = vec![0; (texture.width * texture.height * pixel_size as u32) as usize];
    let mut position = 0;
    let mut output = 0;

    loop {
        let (offset, pixels) = reader(&texture.pixels, position);
        position += offset;

        rgba[output] = pixels.0;
        rgba[output + 1] = pixels.1;
        rgba[output + 2] = pixels.2;
        rgba[output + 3] = pixels.3;

        output += 4;

        if position >= texture.pixels.len() {
            break;
        }
    }

    Texture {
        width: texture.width,
        height: texture.height,
        rgba,
        pixel_size,
    }
}

pub fn to_texture_sampler(textures: &[Arc<Texture>], info: &gltf::Texture) -> TextureSampler {
    TextureSampler {
        texture: textures[info.source().index()].clone(),
        sampler: to_sampler(&info.sampler()),
    }
}

pub fn load_material(material: &gltf::Material, textures: &[Arc<Texture>]) -> Material {
    let pbr = material.pbr_metallic_roughness();

    // Albedo.
    let albedo_factor = Vector3::from_slice(&pbr.base_color_factor()[0..3]);
    let albedo_texture = pbr
        .base_color_texture()
        .map(|info| to_texture_sampler(textures, &info.texture()));

    // Emissive.
    let emitted_factor = Vector3::from_slice(&material.emissive_factor()[0..3]);
    let emitted_texture = material
        .emissive_texture()
        .map(|info| to_texture_sampler(textures, &info.texture()));

    // Normal.
    let normal_texture = material
        .normal_texture()
        .map(|info| to_texture_sampler(textures, &info.texture()));

    // Metalic + roughness.
    let metalic = pbr.metallic_factor();
    let roughness = pbr.roughness_factor();
    let metalic_roughness_texture = pbr
        .metallic_roughness_texture()
        .map(|info| to_texture_sampler(textures, &info.texture()));

    let single_sided = !material.double_sided();

    let alpha_mode = match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.)),
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    };

    Material {
        alpha_mode,
        albedo_factor,
        albedo_texture,
        emitted_factor,
        emitted_texture,
        normal_texture,
        metalic,
        roughness,
        metalic_roughness_texture,
        single_sided,
        brdf: Box::new(MicrofacetBrdf::new()),
    }
}

pub fn load_mesh<H>(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
    material: Arc<Material>,
    transformation: cgmath::Matrix4<f32>,
    material_index: i32,
    handler: &mut Option<&mut H>,
) -> Mesh
where
    H: SceneImportHandler,
{
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    // println!(
    //     "  ------ material: {} {}",
    //     material_index,
    //     primitive.material().name().unwrap_or("<material>")
    // );

    let vertices = match reader.read_positions() {
        Some(iter) => iter.collect(),
        None => vec![],
    };
    // println!("  ------ vertex count: {}", vertices.len());

    let normals = match reader.read_normals() {
        Some(iter) => iter.collect(),
        None => vec![],
    };

    let coords = match reader.read_tex_coords(0) {
        Some(iter) => iter.into_f32().collect(),
        None => vec![],
    };
    // println!("  ------ coord count: {}", coords.len());

    let indices = match reader.read_indices() {
        Some(iter) => iter.into_u32().map(|i| i as usize).collect(),
        None => vec![],
    };
    // println!("  ------ index count: {}", indices.len());

    let mut mesh_triangles = Vec::with_capacity(indices.len() / 3);

    for triangle in indices.chunks(3) {
        let mut vertex = [Vertex::new(); 3];

        for (i, index) in triangle.iter().enumerate() {
            let vert = vertices[*index];
            let coord = if coords.is_empty() {
                [0.; 2]
            } else {
                coords[*index]
            };

            vertex[i] = Vertex {
                pos: Vector3::new(vert[0], vert[1], vert[2]),
                uv: (coord[0], coord[1]),
            };
        }

        mesh_triangles.push(Triangle { vertex });
    }

    if let Some(handler) = handler {
        handle_mesh(
            *handler,
            material_index,
            transformation,
            &vertices,
            &coords,
            &normals,
            &indices,
        );
    }

    Mesh::new(mesh_triangles, material, transformation)
}

pub fn load<H>(
    filename: &Path,
    transformation: cgmath::Matrix4<f32>,
    handler: &mut Option<&mut H>,
) -> Result<Vec<Mesh>, Error>
where
    H: SceneImportHandler,
{
    println!("Loading gltf {:?}...", filename);

    let (gltf, buffers, gltf_textures) = gltf::import(filename)?;

    let textures: Vec<Arc<Texture>> = gltf_textures
        .iter()
        .map(|gltf_texture| Arc::new(to_texture(gltf_texture)))
        .collect();

    let mut meshes: Vec<Mesh> = vec![];
    let mut materials: Vec<Arc<Material>> = vec![];
    let mut material_cache: HashMap<usize, usize> = HashMap::new();

    let dummy_material = Arc::new(Material {
        alpha_mode: AlphaMode::Opaque,
        albedo_factor: Vector3::one(),
        albedo_texture: None,
        emitted_factor: Vector3::zero(),
        emitted_texture: None,
        normal_texture: None,
        metalic: 0.,
        roughness: 0.,
        metalic_roughness_texture: None,
        single_sided: false,
        brdf: Box::new(Dielectric::new(1.5)),
    });

    if handler.is_some() {
        for camera in gltf.cameras() {
            let handler = handler.as_mut().unwrap();
            match camera.projection() {
                gltf::camera::Projection::Orthographic(ortho) => handler.handle_ortho_camera(
                    ortho.xmag(),
                    ortho.ymag(),
                    ortho.znear(),
                    ortho.zfar(),
                ),
                gltf::camera::Projection::Perspective(p) => handler.handle_perspective_camera(
                    p.yfov(),
                    p.aspect_ratio().unwrap_or(1.),
                    p.znear(),
                    p.zfar().unwrap_or(1000.),
                ),
            }
        }
    }

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            // Transform.
            let (translate, rotate, scale) = node.transform().decomposed();

            let node_transform = cgmath::Matrix4::from_translation(Vector3::from_slice(&translate))
                * cgmath::Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2])
                * cgmath::Matrix4::from(cgmath::Quaternion::new(
                    rotate[3], rotate[0], rotate[1], rotate[2],
                ));

            // Camera.
            if let Some(camera) = node.camera() {
                let camera_index = camera.index();

                if handler.is_some() {
                    let handler = handler.as_mut().unwrap();
                    handler.handle_camera_transform(camera_index, &translate, &rotate);
                }
            }

            // Meshes.
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    let (material_index, material) = match primitive.material().index() {
                        Some(source_material) => match material_cache.get(&source_material) {
                            Some(index) => (*index as i32, materials[*index].clone()),
                            None => {
                                let material_index = materials.len();
                                materials.push(Arc::new(load_material(
                                    &primitive.material(),
                                    &textures,
                                )));
                                material_cache.insert(source_material, material_index);
                                let material = materials[material_index].clone();

                                if let Some(handler) = handler {
                                    handle_material(*handler, &material);
                                }

                                (material_index as i32, material)
                            }
                        },
                        _ => (-1, dummy_material.clone()),
                    };

                    meshes.push(load_mesh(
                        &primitive,
                        &buffers,
                        material,
                        transformation * node_transform,
                        material_index,
                        handler,
                    ));
                }
            }
        }
    }

    Ok(meshes)
}
