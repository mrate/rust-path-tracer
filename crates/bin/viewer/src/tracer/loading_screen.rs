use crate::tracer::consts;
use crate::tracer::renderable::Renderable;

use glium::texture::*;
use glium::uniform;

use std::time::Instant;

pub struct LoadingScreen {
    renderable: Renderable,
    texture: SrgbTexture2d,
    start: Instant,
}

impl LoadingScreen {
    pub fn new(display: &glium::Display) -> Self {
        let load_screen_texture = load_screen_texture(display);

        LoadingScreen {
            renderable: Renderable::new(
                display,
                consts::LOAD_SCREEN_VS,
                consts::LOAD_SCREEN_FS,
                &consts::fullscreen_vertices(),
                &consts::fullscreen_indices(),
                false,
            ),
            texture: load_screen_texture,
            start: Instant::now(),
        }
    }

    pub fn render(&self, target: &mut glium::Frame) {
        let seconds = (Instant::now() - self.start).as_secs_f32();

        let matrix: [[f32; 4]; 4] =
            cgmath::Matrix4::from_scale(0.3 + 0.01 * (5. * seconds).sin()).into();

        let uniform = uniform! {
            viewProj: matrix,
            inTexture: &self.texture,
        };

        self.renderable.draw(target, &uniform, false, false);
    }
}

fn load_screen_texture(display: &glium::Display) -> SrgbTexture2d {
    use std::io::Cursor;
    let image = image::load(
        Cursor::new(&include_bytes!("../../assets/load_screen.png")),
        image::ImageFormat::Png,
    )
    .unwrap()
    .to_rgba8();
    let image_dimensions = image.dimensions();
    let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);

    SrgbTexture2d::new(display, image).unwrap()
}
