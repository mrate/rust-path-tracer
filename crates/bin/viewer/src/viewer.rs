mod tracer;
mod window;

use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::ControlFlow;

use std::time::Instant;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;

fn main() {
    let win = window::Window::new("rust-path-tracer", WIDTH as i32, HEIGHT as i32);
    let window::Window {
        event_loop,
        display,
        mut imgui,
        mut platform,
        mut renderer,
        ..
    } = win;

    let mut last_frame = Instant::now();

    let mut tracer = tracer::Tracer::new(&display, WIDTH, HEIGHT);
    tracer.load_scene(std::path::Path::new("./assets/starwars.json").to_owned());

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(_) => {
            let now = Instant::now();
            imgui.io_mut().update_delta_time(now - last_frame);
            last_frame = now;
        }
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            platform
                .prepare_frame(imgui.io_mut(), gl_window.window())
                .expect("Failed to prepare frame");
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            let ui = imgui.frame();
            tracer.update_ui(ui);

            let gl_window = display.gl_window();
            let mut target = display.draw();
            platform.prepare_render(ui, gl_window.window());

            // MAIN RENDER
            tracer.update(&display);
            tracer.render(&mut target);

            let draw_data = imgui.render();
            renderer
                .render(&mut target, draw_data)
                .expect("Rendering failed");
            target.finish().expect("Failed to swap buffers");
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            tracer.exit();
            *control_flow = ControlFlow::Exit;
        }
        event => {
            let gl_window = display.gl_window();
            platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
        }
    });
}
