use crate::tracer::renderable::Renderable;

use glium::texture::Texture2d;
use glium::uniform;

pub struct SceneRenderer {
    pub renderables: Vec<Renderable>,
    pub textures: Vec<Texture2d>,
    pub texture_mapping: Vec<i32>,
}

impl SceneRenderer {
    pub fn render(
        &self,
        target: &mut glium::Frame,
        camera: [[f32; 4]; 4],
        default_texture: &Texture2d,
    ) {
        // Render scene.
        for (i, renderable) in self.renderables.iter().enumerate() {
            let texture = match self.texture_mapping[i as usize] {
                -1 => default_texture,
                i => {
                    // TODO: Material without textures.
                    if (i as usize) < self.textures.len() {
                        &self.textures[i as usize]
                    } else {
                        default_texture
                    }
                }
            };

            let scene_uniforms = uniform! {
                viewProj: camera,
                inTexture: texture,
            };

            renderable.draw(target, &scene_uniforms, false, true);
        }
    }
}
