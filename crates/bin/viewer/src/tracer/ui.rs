use super::image_io::*;
use super::Tracer;

use pathtracer::random::UniformSampler;

use imgui::*;

use native_dialog::{MessageDialog, MessageType};

use std::time::Instant;

pub const CAMERA_JSON: &str = "camera.json";
pub const SETTINGS_JSON: &str = "settings.json";

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum UserState {
    Moving,
    Rendering,
    Loading,
}

#[derive(Debug, Copy, Clone)]
pub struct UiSettings {
    pub state: UserState,
    pub visible: bool,
    pub show_info: bool,
    pub show_imgui: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            state: UserState::Rendering,
            visible: true,
            show_info: Default::default(),
            show_imgui: Default::default(),
        }
    }
}

pub fn show_error(err: &str) {
    MessageDialog::new()
        .set_type(MessageType::Error)
        .set_title("Error")
        .set_text(err)
        .show_alert()
        .unwrap();
}

pub fn show_info(title: &str, info: &str) {
    MessageDialog::new()
        .set_type(MessageType::Info)
        .set_title(title)
        .set_text(info)
        .show_alert()
        .unwrap();
}

fn update_mouse(tracer: &mut Tracer, io: &imgui::Io) {
    if tracer.right_mouse && !io.mouse_down[1] {
        let x = io.mouse_pos[0] as f32 / tracer.width as f32;
        let y = io.mouse_pos[1] as f32 / tracer.height as f32;
        let ray = tracer
            .camera
            .tracer_simple_camera()
            .ray(x, 1. - y, &UniformSampler::new());

        let distance = tracer
            .tracer
            .read()
            .unwrap()
            .scene()
            .hit(&ray, 0.001, std::f32::INFINITY)
            .map_or(0., |hit| hit.t);

        if distance > 0. {
            tracer.camera.focus_distance = distance;
            tracer.reset_tracing();
        }
    }
    tracer.right_mouse = io.mouse_down[1];

    match tracer.ui.state {
        UserState::Moving => {
            if io.mouse_down[0] {
                let (x, y) = (io.mouse_delta[0], io.mouse_delta[1]);

                if x != 0. || y != 0. {
                    tracer.reset_pending = true;

                    tracer.camera.yaw -= f32::to_radians(0.5 * x);
                    tracer.camera.pitch += f32::to_radians(0.5 * y);
                    tracer.camera.pitch = tracer
                        .camera
                        .pitch
                        .max(-0.49 * std::f32::consts::PI)
                        .min(0.49 * std::f32::consts::PI);
                }
            } else {
                tracer.ui.state = UserState::Rendering;

                tracer.apply_reset_pending();
            }
        }
        UserState::Rendering => {
            if io.mouse_down[0] {
                tracer.ui.state = UserState::Moving;
            }
        }

        _ => (),
    }
}

fn update_keyboard(tracer: &mut Tracer) {
    unsafe {
        if imgui_sys::igIsKeyPressed(0x29, false) {
            let _ = tracer.camera.save(CAMERA_JSON);
        }

        if imgui_sys::igIsKeyPressed(0x2D, false) {
            match tracer.camera.load(CAMERA_JSON) {
                Ok(_) => tracer.reset_tracing(),
                Err(err) => println!("Failed to load camera: {:?}", err),
            }
        }

        if imgui_sys::igIsKeyPressed(0x70, false) {
            tracer.ui.visible = !tracer.ui.visible;
        }
    }

    if tracer.ui.state == UserState::Moving {
        let mut moved = false;
        let mut dir = cgmath::Vector3::new(0., 0., 0.);

        unsafe {
            if imgui_sys::igIsKeyDown(0x20) {
                moved = true;
                dir += tracer.camera.forward();
            }
            if imgui_sys::igIsKeyDown(0x1c) {
                moved = true;
                dir += tracer.camera.forward() * -1.;
            }
            if imgui_sys::igIsKeyDown(0xd) {
                moved = true;
                dir += tracer.camera.right() * -1.;
            }
            if imgui_sys::igIsKeyDown(0xa) {
                moved = true;
                dir += tracer.camera.right();
            }
            if imgui_sys::igIsKeyDown(0x1a) {
                moved = true;
                dir += tracer.camera.up() * -1.;
            }
            if imgui_sys::igIsKeyDown(0xe) {
                moved = true;
                dir += tracer.camera.up();
            }
        };

        if moved {
            tracer.camera.move_offset(&dir);
            tracer.reset_pending = true;
        }
    }
}

pub fn generate_main_menu(tracer: &mut Tracer, ui: &Ui) {
    if let Some(_main_menu) = ui.begin_main_menu_bar() {
        menu_file(ui, tracer);
        menu_settings(ui, tracer);
        menu_view(ui, tracer);
        menu_camera(ui, tracer);
    }
}

fn menu_camera(ui: &Ui, tracer: &mut Tracer) {
    if let Some(_menu) = ui.begin_menu("Camera") {
        if ui.selectable("Reset") {
            tracer.camera.reset();
        }

        if ui.selectable("Load") {
            match tracer.camera.load(CAMERA_JSON) {
                Ok(_) => tracer.reset_tracing(),
                Err(err) => show_error(&format!("Failed to load camera: {:?}", err)),
            }
        }

        if ui.selectable("Save") {
            let _ = tracer.camera.save(CAMERA_JSON);
        }

        ui.separator();

        ui.slider("Movement speed", 0.01, 5., &mut tracer.camera.speed);
        ui.slider("Exposure", 0., 10., &mut tracer.log_exposure);
        ui.separator();

        let mut modified = false;

        modified = ui.slider("Vertical fov", 30., 160., &mut tracer.camera.v_fov) || modified;
        modified =
            ui.radio_button("Simple camera", &mut tracer.camera.simple_camera, true) || modified;
        modified =
            ui.radio_button("Aperture camera", &mut tracer.camera.simple_camera, false) || modified;
        modified = ui.slider("Aperture", 0.1, 10., &mut tracer.camera.aperture) || modified;
        modified = ui.slider(
            "Focus distance",
            0.01,
            100000.,
            &mut tracer.camera.focus_distance,
        ) || modified;

        if modified {
            tracer.reset_tracing();
        }
    }
}

fn menu_view(ui: &Ui, tracer: &mut Tracer) {
    if let Some(_menu) = ui.begin_menu("View") {
        ui.checkbox("Info", &mut tracer.ui.show_info);
        ui.separator();
        ui.checkbox("ImGui demo", &mut tracer.ui.show_imgui);
    }
}

fn menu_settings(ui: &Ui, tracer: &mut Tracer) {
    if let Some(_menu) = ui.begin_menu("Settings") {
        let mut modified = false;

        modified = ui.checkbox("Shadow rays", &mut tracer.tracer_settings.shadow_rays) || modified;
        modified = ui.checkbox(
            "Random light sample",
            &mut tracer.tracer_settings.random_light_sample,
        ) || modified;
        modified = ui.slider(
            "Bounces",
            1,
            10,
            &mut tracer.tracer_settings.max_scatter_depth,
        ) || modified;

        if modified {
            tracer.reset_tracing();
            let _ = tracer.tracer_settings.save(SETTINGS_JSON);
        }
    }
}

fn menu_file(ui: &Ui, tracer: &mut Tracer) {
    if let Some(_menu) = ui.begin_menu("File") {
        if ui.selectable("Load scene...") {
            let path = get_open_file_name("Scene", &["json"]);
            if let Some(path) = path {
                tracer.load_scene(path);
                tracer.set_cancel(true);
                println!("Loading scene, waiting for frame to finish...");
            }
        }

        if ui.selectable("Import...") {
            let path = get_open_file_name("pfm image", &["pfm"]);
            if let Some(path) = path {
                tracer.add_pending_action(move |tracer: &mut Tracer, _| tracer.import_image(&path));

                tracer.set_cancel(true);
                println!("Loading output, waiting for frame to finish...");
            }
        }

        if ui.selectable("Export...") {
            let path = get_save_file_name("pfm image", &["pfm"]);
            if let Some(path) = path {
                tracer.add_pending_action(move |tracer: &mut Tracer, _| tracer.export_image(&path));

                println!("Saving output, waiting for frame to finish...");
            }
        }
    }
}

fn info_window(tracer: &mut Tracer, ui: &Ui) {
    ui.window("Info")
        .size([500.0, 200.0], Condition::FirstUseEver)
        .opened(&mut tracer.ui.show_info)
        .build(|| {
            let camera_pos = tracer.camera.position();
            ui.text_wrapped(format!(
                "Camera: {:.2}, {:.2}, {:.2} (yaw={:.2}, pitch={:.2})",
                camera_pos.x, camera_pos.y, camera_pos.z, tracer.camera.yaw, tracer.camera.pitch
            ));

            ui.separator();

            let frame_time = Instant::now() - tracer.frame_start;
            let render_time = Instant::now() - tracer.render_start;

            ui.text_wrapped(format!("SPP: {}", tracer.average.sample()));
            ui.text_wrapped(format!(
                "Frame time: {:.2}s",
                (frame_time.as_millis() as f32) / 1000.0
            ));
            ui.text_wrapped(format!(
                "Total time: {:.1}m",
                (render_time.as_millis() as f32) / 60000.0
            ));
            ui.text_wrapped(format!(
                "Average frame time: {:.2}s",
                tracer.average_frame_seconds
            ));

            ui.text_wrapped(format!(
                "Progress: {:.1}%",
                (1. - tracer.pending.0 as f32 / tracer.pending.1 as f32) * 100.
            ));
        });
}

pub fn generate_ui(tracer: &mut Tracer, ui: &Ui) {
    if !ui.io().want_capture_mouse {
        update_mouse(tracer, ui.io());
    }

    if !ui.io().want_capture_keyboard {
        update_keyboard(tracer);
    }

    if tracer.ui.visible {
        generate_main_menu(tracer, ui);

        if tracer.ui.show_info {
            info_window(tracer, ui);
        }

        if tracer.ui.show_imgui {
            ui.show_demo_window(&mut tracer.ui.show_imgui);
        }
    }
}
