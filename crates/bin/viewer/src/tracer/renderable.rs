use glium::*;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

implement_vertex!(Vertex, position, uv, normal);

pub struct Renderable {
    program: glium::Program,
    vertex_buffer: glium::VertexBuffer<Vertex>,
    indices: glium::index::IndexBuffer<u32>,
}

//impl Sync for Renderable;
unsafe impl Send for Renderable {}

impl Renderable {
    pub fn new(
        display: &glium::Display,
        vs: &str,
        fs: &str,
        vertices: &[Vertex],
        indices: &[u32],
        srgb: bool,
    ) -> Renderable {
        let input = glium::program::ProgramCreationInput::SourceCode {
            vertex_shader: vs,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: fs,
            transform_feedback_varyings: None,
            outputs_srgb: srgb,
            uses_point_size: false,
        };

        let program = glium::Program::new(display, input).unwrap();
        let vertex_buffer = glium::VertexBuffer::new(display, vertices).unwrap();
        let indices = glium::index::IndexBuffer::new(
            display,
            glium::index::PrimitiveType::TrianglesList,
            indices,
        )
        .unwrap();

        Renderable {
            program,
            vertex_buffer,
            indices,
        }
    }

    pub fn draw<U>(
        &self,
        target: &mut glium::Frame,
        uniforms: &U,
        wireframe: bool,
        depth_test: bool,
    ) where
        U: uniforms::Uniforms,
    {
        let params = glium::DrawParameters {
            line_width: Some(1.0),
            polygon_mode: if wireframe {
                glium::PolygonMode::Line
            } else {
                glium::PolygonMode::Fill
            },
            blend: glium::Blend::alpha_blending(),
            depth: if depth_test {
                glium::Depth {
                    test: glium::DepthTest::IfLess,
                    write: true,
                    ..Default::default()
                }
            } else {
                glium::Depth {
                    test: glium::DepthTest::Overwrite,
                    write: false,
                    ..Default::default()
                }
            },
            ..Default::default()
        };

        target
            .draw(
                &self.vertex_buffer,
                &self.indices,
                &self.program,
                uniforms,
                &params,
            )
            .unwrap();
    }
}
