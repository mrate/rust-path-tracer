use glium::glutin;
use glium::glutin::event_loop::EventLoop;
use glium::glutin::window::WindowBuilder;
use glium::Display;
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use std::path::Path;

mod clipboard;
pub struct Window {
    pub event_loop: EventLoop<()>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
}

impl Window {
    pub fn new(title: &str, width: i32, height: i32) -> Window {
        let title = match Path::new(&title).file_name() {
            Some(file_name) => file_name.to_str().unwrap(),
            None => title,
        };
        let event_loop = EventLoop::new();
        let context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_depth_buffer(24);
        let builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(glutin::dpi::LogicalSize::new(width as f64, height as f64));
        let display =
            Display::new(builder, context, &event_loop).expect("Failed to initialize display");

        let mut imgui = Context::create();
        //imgui.set_ini_filename(None);

        if let Some(backend) = clipboard::init() {
            imgui.set_clipboard_backend(backend);
        } else {
            eprintln!("Failed to initialize clipboard");
        }

        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let gl_window = display.gl_window();
            let window = gl_window.window();

            let dpi_mode = if let Ok(factor) = std::env::var("IMGUI_EXAMPLE_FORCE_DPI_FACTOR") {
                // Allow forcing of HiDPI factor for debugging purposes
                match factor.parse::<f64>() {
                    Ok(f) => HiDpiMode::Locked(f),
                    Err(e) => panic!("Invalid scaling factor: {}", e),
                }
            } else {
                HiDpiMode::Default
            };

            platform.attach_window(imgui.io_mut(), window, dpi_mode);
        }

        // Fixed font size. Note imgui_winit_support uses "logical
        // pixels", which are physical pixels scaled by the devices
        // scaling factor. Meaning, 13.0 pixels should look the same size
        // on two different screens, and thus we do not need to scale this
        // value (as the scaling is handled by winit)
        let font_size = 13.0;

        imgui.fonts().add_font(&[
            FontSource::TtfData {
                data: include_bytes!("./fonts/Roboto-Regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    // As imgui-glium-renderer isn't gamma-correct with
                    // it's font rendering, we apply an arbitrary
                    // multiplier to make the font a bit "heavier". With
                    // default imgui-glow-renderer this is unnecessary.
                    rasterizer_multiply: 1.5,
                    // Oversampling font helps improve text rendering at
                    // expense of larger font atlas texture.
                    oversample_h: 4,
                    oversample_v: 4,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("./fonts/mplus-1p-regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    // Oversampling font helps improve text rendering at
                    // expense of larger font atlas texture.
                    oversample_h: 4,
                    oversample_v: 4,
                    // Range of glyphs to rasterize
                    glyph_ranges: FontGlyphRanges::japanese(),
                    ..FontConfig::default()
                }),
            },
        ]);

        let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

        Window {
            event_loop,
            display,
            imgui,
            platform,
            renderer,
            font_size,
        }
    }
}
