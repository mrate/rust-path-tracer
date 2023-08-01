mod camera_controller;
mod consts;
mod image_io;
mod loading_screen;
mod renderable;
mod scene_importer;
mod scene_renderer;
mod texture_data;
mod ui;

use camera_controller::CameraController;
use image_io::*;
use loading_screen::LoadingScreen;
use scene_importer::ImportHandler;
use scene_renderer::SceneRenderer;
use texture_data::{create_solid_color_texture, TextureBlock, TextureData};
use ui::*;

use glium::texture::Texture2d;
use glium::{uniform, Surface};

use imgui::*;

use ::pathtracer::math::{EnhancedVector, Vector3};
use ::pathtracer::random::UniformSampler;
use ::pathtracer::*;

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use std::time::Instant;

use rand::prelude::*;

const BLOCK_WIDTH: u32 = 32;
const BLOCK_HEIGHT: u32 = 32;

const UPDATE_COUNT: usize = 2048;

enum TracingOutput {
    Output,
    Albedo,
    Normals,
}

type Action = dyn FnMut(&mut Tracer, &glium::Display) -> bool;

pub struct Tracer {
    width: u32,
    height: u32,
    pending: (u32, u32),
    average: math::Average,
    reset_pending: bool,
    reset: bool,
    cancel: Arc<AtomicBool>,
    frame_start: Instant,
    average_frame_seconds: f32,
    render_start: Instant,
    pool: threadpool::ThreadPool,
    camera: camera_controller::CameraController,
    tracing_renderable: renderable::Renderable,
    texture: TextureData,
    tracer: Arc<RwLock<pathtracer::Tracer>>,
    tracer_settings: pathtracer::TracerSettings,
    rx: Receiver<(u32, u32, TextureBlock)>,
    tx: Sender<(u32, u32, TextureBlock)>,
    ui: UiSettings,
    preview: SceneRenderer,
    default_texture: Texture2d,
    loading_screen: LoadingScreen,
    log_exposure: f32,
    right_mouse: bool,
    albedo: TextureData,
    has_albedo: bool,
    normals: TextureData,
    has_normals: bool,
    tracing_output: TracingOutput,
    pending_action: VecDeque<Box<Action>>,
}

impl Tracer {
    pub fn new(display: &glium::Display, width: u32, height: u32) -> Tracer {
        let tracer_settings = pathtracer::TracerSettings::from_file(SETTINGS_JSON).unwrap_or(
            pathtracer::TracerSettings {
                max_scatter_depth: 5,
                shadow_rays: true,
                random_light_sample: false,
                t_min: 0.001,
                t_max: 100000.0,
                min_bounces: 3,
            },
        );

        let mut camera = CameraController::new(
            tracer_settings.t_min,
            tracer_settings.t_max,
            (width as f32) / (height as f32),
            90.0,
        );
        let _ = camera.load(CAMERA_JSON);

        let tracer = Arc::new(RwLock::new(pathtracer::Tracer::new(
            camera.tracer_camera(),
            scene::Scene::empty(),
            tracer_settings,
        )));

        let texture = TextureData::new(display, width, height);
        let albedo = TextureData::new(display, width, height);
        let normals = TextureData::new(display, width, height);

        let tracing_renderable = renderable::Renderable::new(
            display,
            consts::SCREEN_SPACE_VS,
            consts::TEXTURE_FS,
            &consts::fullscreen_vertices(),
            &consts::fullscreen_indices(),
            true,
        );

        let pool = threadpool::ThreadPool::new(None);
        let (tx, rx) = channel();

        let preview = SceneRenderer {
            renderables: vec![],
            textures: vec![],
            texture_mapping: vec![],
        };

        let default_texture = create_solid_color_texture(display, (255, 255, 255, 255));

        Tracer {
            width,
            height,
            pending: (0, 0),
            average: Default::default(),
            reset_pending: false,
            reset: false,
            cancel: Arc::new(AtomicBool::new(false)),
            frame_start: Instant::now(),
            average_frame_seconds: 0.,
            render_start: Instant::now(),
            pool,
            camera,
            preview,
            default_texture,
            loading_screen: LoadingScreen::new(display),
            tracing_renderable,
            texture,
            tracer,
            tracer_settings,
            rx,
            tx,
            ui: Default::default(),
            log_exposure: 1.0,
            right_mouse: false,
            albedo,
            has_albedo: false,
            normals,
            has_normals: false,
            tracing_output: TracingOutput::Output,
            pending_action: VecDeque::new(),
        }
    }

    fn spawn_threads(&self, grid_width: u32, grid_height: u32) -> u32 {
        let mut spawned = 0;

        for grid_y in 0..grid_height {
            for grid_x in 0..grid_width {
                let thread_tx = self.tx.clone();
                let thread_tracer = self.tracer.clone();
                let thread_cancel = self.cancel.clone();

                let (width, height) = (self.width as f32, self.height as f32);

                let work = move || {
                    let mut block = TextureBlock::new(BLOCK_WIDTH, BLOCK_HEIGHT);

                    for block_y in 0..BLOCK_HEIGHT {
                        for block_x in 0..BLOCK_WIDTH {
                            let (x, y) = (
                                grid_x * BLOCK_WIDTH + block_x,
                                grid_y * BLOCK_HEIGHT + block_y,
                            );

                            let norm_x = ((x as f32) + random::<f32>()) / width;
                            let norm_y = ((y as f32) + random::<f32>()) / height;
                            let color =
                                match thread_cancel.load(std::sync::atomic::Ordering::Acquire) {
                                    true => math::Vector3::zero(),
                                    false => thread_tracer.read().unwrap().trace(norm_x, norm_y),
                                };

                            block.set(block_x, block_y, color);
                        }
                    }

                    thread_tx
                        .send((grid_x * BLOCK_WIDTH, grid_y * BLOCK_HEIGHT, block))
                        .expect("should be able to send block");
                };

                self.pool.schedule(work);
                spawned += 1;
            }
        }

        spawned
    }

    fn spawn_threads_albedo_normals(
        &self,
        grid_width: u32,
        grid_height: u32,
        normals: bool,
    ) -> u32 {
        let mut spawned = 0;

        for grid_y in 0..grid_height {
            for grid_x in 0..grid_width {
                let thread_tx = self.tx.clone();
                let thread_tracer = self.tracer.clone();
                let thread_cancel = self.cancel.clone();

                let (width, height) = (self.width as f32, self.height as f32);
                let tracer_camera = self.camera.tracer_simple_camera();
                let (t_min, t_max) = (self.tracer_settings.t_min, self.tracer_settings.t_max);

                let work = move || {
                    let mut block = TextureBlock::new(BLOCK_WIDTH, BLOCK_HEIGHT);

                    for block_y in 0..BLOCK_HEIGHT {
                        for block_x in 0..BLOCK_WIDTH {
                            let (x, y) = (
                                grid_x * BLOCK_WIDTH + block_x,
                                grid_y * BLOCK_HEIGHT + block_y,
                            );

                            let norm_x = (x as f32) / width;
                            let norm_y = (y as f32) / height;
                            let albedo_normals = match thread_cancel
                                .load(std::sync::atomic::Ordering::Acquire)
                            {
                                true => (math::Vector3::zero(), math::Vector3::zero()),
                                false => {
                                    match thread_tracer.read().unwrap().scene().hit(
                                        &tracer_camera.ray(norm_x, norm_y, &UniformSampler::new()),
                                        t_min,
                                        t_max,
                                    ) {
                                        Some(hit) => (hit.material.base_color(hit.uv), hit.normal),
                                        None => (math::Vector3::zero(), math::Vector3::zero()),
                                    }
                                }
                            };

                            block.set(
                                block_x,
                                block_y,
                                if normals {
                                    albedo_normals.1
                                } else {
                                    albedo_normals.0
                                },
                            );
                        }
                    }

                    thread_tx
                        .send((grid_x * BLOCK_WIDTH, grid_y * BLOCK_HEIGHT, block))
                        .expect("should be able to send block");
                };

                self.pool.schedule(work);
                spawned += 1;
            }
        }

        spawned
    }

    fn reset_tracing(&mut self) {
        self.reset_pending = false;
        self.reset = true;
        self.set_cancel(true);
    }

    fn apply_reset_pending(&mut self) {
        if self.reset_pending {
            self.reset_tracing()
        }
    }

    fn set_cancel(&self, value: bool) {
        self.cancel
            .store(value, std::sync::atomic::Ordering::Release);
    }

    fn begin_tracing(&mut self) {
        self.reset = false;
        self.set_cancel(false);
        self.average.reset();
        self.render_start = Instant::now();
        self.average_frame_seconds = 0.;
    }

    fn reset(&self) -> bool {
        self.reset
    }

    fn set_scene(&mut self, display: &glium::Display, scene: scene::Scene, handler: ImportHandler) {
        self.tracer.write().unwrap().set_scene(scene);

        self.reset_tracing();
        self.has_albedo = false;
        self.has_normals = false;

        let (renderables, textures, texture_mapping) =
            handler.generate(display, consts::SCENE_VS, consts::SCENE_FS);

        self.preview = SceneRenderer {
            renderables,
            textures,
            texture_mapping,
        };
    }

    pub fn load_scene(&mut self, path: std::path::PathBuf) {
        self.ui.state = UserState::Loading;

        let (tx, rx) = channel();

        let mut thread = Some(std::thread::spawn(move || {
            let mut handler = ImportHandler::new();
            let result = scene::Scene::load(&path, &mut Some(&mut handler));
            tx.send((handler, result)).unwrap();
        }));

        self.add_pending_action(move |tracer: &mut Tracer, display: &glium::Display| {
            if let Some((handler, result)) = rx.try_iter().next() {
                let t = thread.take();
                t.unwrap().join().unwrap();

                match result {
                    Err(err) => show_error(&format!("Failed to load scene: {:?}", err)),
                    Ok(scene) => tracer.set_scene(display, scene, handler),
                }

                println!("Loaded");
                tracer.ui.state = UserState::Rendering;
                true
            } else {
                false
            }
        });
    }

    pub fn update_ui(&mut self, ui: &Ui) {
        generate_ui(self, ui);
    }

    fn export_image(&mut self, path: &std::path::Path) -> bool {
        if !self.has_albedo {
            println!("Missing albedo, tracing...");

            let spawned = self.spawn_threads_albedo_normals(
                self.width / BLOCK_WIDTH,
                self.height / BLOCK_HEIGHT,
                false,
            );
            self.pending = (spawned, spawned);
            self.tracing_output = TracingOutput::Albedo;
            self.has_albedo = true;

            return false;
        }

        if !self.has_normals {
            println!("Missing normals, tracing...");

            let spawned = self.spawn_threads_albedo_normals(
                self.width / BLOCK_WIDTH,
                self.height / BLOCK_HEIGHT,
                true,
            );
            self.pending = (spawned, spawned);
            self.tracing_output = TracingOutput::Normals;
            self.has_normals = true;

            return false;
        }

        self.tracing_output = TracingOutput::Output;
        let path_base = add_suffix(path, &format!("_{}spp", self.average.sample()));
        let path_albedo = add_suffix(path, &format!("_{}spp_albedo", self.average.sample()));
        let path_normals = add_suffix(path, &format!("_{}spp_normals", self.average.sample()));
        let mut path_denoise = add_suffix(path, "_denoise");
        path_denoise.set_extension("ps1");

        println!("Saving {}...", path_base.to_str().unwrap());
        save_pfm(&path_base, self.width, self.height, &self.texture.data).unwrap_or_else(|err| {
            show_error(&format!(
                "Failed to save {}: {}",
                path_base.to_str().unwrap(),
                err
            ))
        });

        println!("Saving {}...", path_albedo.to_str().unwrap());
        save_pfm(&path_albedo, self.width, self.height, &self.albedo.data).unwrap_or_else(|err| {
            show_error(&format!(
                "Failed to save {}: {}",
                path_albedo.to_str().unwrap(),
                err
            ))
        });

        println!("Saving {}...", path_normals.to_str().unwrap());
        save_pfm(&path_normals, self.width, self.height, &self.normals.data).unwrap_or_else(
            |err| {
                show_error(&format!(
                    "Failed to save {}: {}",
                    path_normals.to_str().unwrap(),
                    err
                ))
            },
        );

        println!("Saving {}...", path_denoise.to_str().unwrap());
        save_denoise(&path_denoise, &path_base, &path_albedo, &path_normals).unwrap_or_else(
            |err| {
                show_error(&format!(
                    "Failed to save {}: {}",
                    path_denoise.to_str().unwrap(),
                    err
                ))
            },
        );

        show_info("Info", "Output saved.");

        true
    }

    fn import_image(&mut self, _path: &std::path::Path) -> bool {
        show_error("Cannot load pfm, need to save whole state (scene file, camera settings, etc. first)...");

        //let sample = parse_sample(path.file_stem().unwrap().to_str().unwrap());
        // println!(
        //     "Loading file: '{}' with sample '{}'",
        //     path.to_str().unwrap(),
        //     sample
        // );

        // match load_pfm(&path) {
        //     Err(err) => show_error(&format!(
        //         "Failed to load '{}': {}",
        //         path.to_str().unwrap(),
        //         err.to_string()
        //     )),
        //     Ok((width, height, data)) => {
        //         if self.width == width && self.height == height {
        //             self.texture.set_data(data);
        //             self.average.set_sample(1);
        //         }
        //     }
        // }

        true
    }

    fn add_pending_action<F>(&mut self, f: F)
    where
        F: FnMut(&mut Tracer, &glium::Display) -> bool + 'static,
    {
        self.pending_action.push_back(Box::new(f));
    }

    fn handle_pending_action(&mut self, display: &glium::Display) {
        if let Some(mut action) = self.pending_action.pop_front() {
            if !action(self, display) {
                self.pending_action.push_front(action);
            }
        }
    }

    fn start_new_sample(&mut self) {
        if self.reset() {
            self.begin_tracing();
            self.texture.clear();

            self.tracer
                .write()
                .unwrap()
                .set_camera(self.camera.tracer_camera());

            self.tracer
                .write()
                .unwrap()
                .set_settings(self.tracer_settings);
        }

        let frame_seconds = (Instant::now() - self.frame_start).as_secs_f32();
        if self.average.sample() > 0 {
            self.average_frame_seconds = self
                .average
                .average(self.average_frame_seconds, frame_seconds);
        }

        self.frame_start = Instant::now();
        self.average.next_frame();

        let spawned = self.spawn_threads(self.width / BLOCK_WIDTH, self.height / BLOCK_HEIGHT);
        self.pending = (spawned, spawned);
    }

    fn update_raytracing_texture(&mut self) {
        for (_, (x, y, block)) in self.rx.try_iter().enumerate().take(UPDATE_COUNT) {
            let mut local_data = Vec::with_capacity((block.width * block.height * 4) as usize);

            for block_y in 0..block.height {
                for block_x in 0..block.width {
                    let index = (x + block_x + (y + block_y) * self.width) as usize;

                    let color = match self.tracing_output {
                        TracingOutput::Albedo => {
                            self.albedo.data[index] = *block.get(block_x, block_y);
                            self.albedo.data[index]
                        }
                        TracingOutput::Normals => {
                            let normal = *block.get(block_x, block_y);
                            self.normals.data[index] = normal;
                            normal * 2. - Vector3::one()
                        }
                        TracingOutput::Output => {
                            self.texture.data[index] = self
                                .average
                                .average(self.texture.data[index], *block.get(block_x, block_y));
                            self.texture.data[index]
                        }
                    };

                    local_data.push(color.x);
                    local_data.push(color.y);
                    local_data.push(color.z);
                    local_data.push(1.);
                }
            }

            match self.tracing_output {
                TracingOutput::Albedo => &mut self.albedo,
                TracingOutput::Normals => &mut self.normals,
                TracingOutput::Output => &mut self.texture,
            }
            .sync(local_data, (x, y, block.width, block.height));

            self.pending.0 -= 1;
        }
    }

    pub fn update(&mut self, display: &glium::Display) {
        if self.pending.0 == 0 && self.ui.state != UserState::Moving {
            self.handle_pending_action(display);
            if self.pending_action.is_empty() {
                self.start_new_sample();
            }
        } else {
            self.update_raytracing_texture();
        }
    }

    pub fn render(&self, target: &mut glium::Frame) {
        if self.ui.state == UserState::Loading {
            target.clear_color_srgb_and_depth((0., 0., 0., 0.), 1.0);
            self.loading_screen.render(target);
        } else {
            target.clear_color_srgb_and_depth((0.7, 0.6, 0.5, 1.0), 1.0);

            self.preview
                .render(target, self.camera.gl_camera(), &self.default_texture);

            // Render tracing output on top.
            if self.ui.state == UserState::Rendering && !self.reset {
                let tracing_uniforms = uniform! {
                    inTexture: match self.tracing_output {
                        TracingOutput::Output => &self.texture.texture,
                        TracingOutput::Albedo => &self.albedo.texture,
                        TracingOutput::Normals => &self.normals.texture,
                    },
                    exposure: self.log_exposure,
                };

                self.tracing_renderable
                    .draw(target, &tracing_uniforms, false, false)
            }
        }
    }

    pub fn exit(&mut self) {
        self.set_cancel(true);
    }
}
