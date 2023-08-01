use super::renderable;

pub const SCREEN_SPACE_VS: &str = r#"
#version 330 core
layout (location = 0) in vec3 position;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 normal;

out vec2 TexCoord;

void main() {
    gl_Position = vec4(position, 1.0);
    TexCoord = uv;
}
"#;

pub const TEXTURE_FS: &str = r#"
#version 330 core
out vec4 FragColor;
  
in vec2 TexCoord;

uniform sampler2D inTexture;
uniform float exposure;

void main() {
    const float gamma = 2.2;
    vec4 hdrColor = texture(inTexture, TexCoord).rgba;
  
    // exposure tone mapping
    vec3 mapped = vec3(1.0) - exp(-hdrColor.rgb * exposure);
    // gamma correction 
    mapped = pow(mapped, vec3(1.0 / gamma));
  
    FragColor = vec4(mapped, hdrColor.a);
}
"#;

pub const SCENE_VS: &str = r#"
#version 330 core
layout (location = 0) in vec3 position;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 normal;

uniform mat4 viewProj;

out vec2 TexCoord;
out vec3 Normal;

void main() {
    gl_Position = viewProj * vec4(position, 1.0);
    TexCoord = uv;
    Normal = normal;
}
"#;

pub const SCENE_FS: &str = r#"
#version 330 core

in vec2 TexCoord;
in vec3 Normal;

uniform sampler2D inTexture;

out vec4 FragColor;

void main() {
    vec3 lightDir = vec3(0.5, -0.5, 0.5);

    vec4 color = texture(inTexture, TexCoord).rgba;

    if (color.a < 0.5) {
        discard;
    }
    
    float diff = max(dot(Normal, lightDir), 0.25);
  
    FragColor = vec4(diff * color.rgb, color.a);
}
"#;

pub const LOAD_SCREEN_VS: &str = r#"
#version 330 core
layout (location = 0) in vec3 position;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 normal;

uniform mat4 viewProj;

out vec2 TexCoord;

void main() {
    gl_Position = viewProj * vec4(position, 1.0);
    TexCoord = uv;
}
"#;

pub const LOAD_SCREEN_FS: &str = r#"
#version 330 core

in vec2 TexCoord;

uniform sampler2D inTexture;

out vec4 FragColor;

void main() {
    FragColor = texture(inTexture, TexCoord);
}
"#;

pub fn fullscreen_vertices() -> Vec<renderable::Vertex> {
    vec![
        renderable::Vertex {
            position: [1., 1., 0.],
            uv: [1., 1.],
            normal: [0., 0., 1.],
        },
        renderable::Vertex {
            position: [1., -1., 0.],
            uv: [1., 0.],
            normal: [0., 0., 1.],
        }, //
        renderable::Vertex {
            position: [-1., -1., 0.],
            uv: [0., 0.],
            normal: [0., 0., 1.],
        }, //
        renderable::Vertex {
            position: [-1., 1., 0.],
            uv: [0., 1.],
            normal: [0., 0., 1.],
        }, //
    ]
}

pub fn fullscreen_indices() -> Vec<u32> {
    vec![
        0, 1, 3, // first triangle
        1, 2, 3, // second triangle
    ]
}
