//! The single player scene implementation.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use glam::{IVec3, Mat4, UVec2, UVec4, Vec2, Vec3, Vec4};
use glow::HasContext;
use mp3d_core::{textcomponent::TextComponent, world::chunk::CHUNK_SIZE};

use crate::{
    abs::{Mesh, ShaderProgram, Texture, framebuffer::Framebuffer},
    client::{Client, Connection, CurrentGUI, LocalConnection},
    render::{
        clouds::CloudRenderer,
        meshing::mesh_world,
        particles::ParticleSystem,
        profiler::Profiler,
        ui::{
            uirenderer::{DrawCommand, UIRenderMode, UIRenderer},
            widgets::*,
        },
    },
    scenes::{Assets, SceneUpdateContext},
    shader_program,
};

const DEFAULT_UV_RECT: [Vec2; 2] = [Vec2::ZERO, Vec2::ONE];

const FPS_HISTORY_LEN: usize = 120;
const FPS_GRAPH_WIDTH: f32 = 500.0;
const FPS_GRAPH_HEIGHT: f32 = 200.0;
const FPS_GRAPH_Y: f32 = 10.0;

const PROFILER_GRAPH_WIDTH: f32 = 400.0;

const CROSSHAIR_SIZE: f32 = 20.0;
const CROSSHAIR_THICKNESS: f32 = 2.0;
const CROSSHAIR_COLOR: Vec4 = Vec4::new(1.0, 1.0, 1.0, 0.8);

struct SinglePlayerUI {
    chat_input_label: Label,
    pause_screen: Column,
    inventory: Stack,
    hotbar: Row,
    debug_opened: bool,
    fps_timer: f32,
    fps: f32,
    fps_history: [f32; FPS_HISTORY_LEN],
}

struct WorldRenderer {
    chunk_meshes: HashMap<IVec3, Mesh>,
    chunk_mesh_pool: Vec<Mesh>,
    cloud_renderer: CloudRenderer,
    particle_system: ParticleSystem,
    framebuffer: Framebuffer,

    chunk_shader: ShaderProgram,
    entity_shader: ShaderProgram,
    postprocess_shader: ShaderProgram,
    chunk_border_shader: ShaderProgram,

    entity_model: Mesh,
    fullscreen_quad: Mesh,
    cube_wireframe: Mesh,

    pink_black: Texture,

    profiler: Profiler,
}

/// The [`SinglePlayer`] struct represents the single player scene.
pub struct SinglePlayer {
    client: Client<LocalConnection>,
    renderer: WorldRenderer,
    screen_size: UVec2,
    tick_acc: f32,
    tick_rate: f32,
    ui: SinglePlayerUI,
    world_path: PathBuf,
    mouse_pos: Vec2,
    timer: f32,
}

impl SinglePlayer {
    /// Creates a new [`SinglePlayer`] instance.
    pub fn new(
        gl: &Arc<glow::Context>,
        assets: &Arc<Assets>,
        window_size: (u32, u32),
        seed: i32,
        world_path: PathBuf,
        username: String,
    ) -> Self {
        let server = mp3d_core::server::Server::new(true, seed, world_path.clone());
        Self::setup(server, gl, assets, window_size, world_path, username)
    }

    /// Loads a world from the given path and creates a new [`SinglePlayer`] instance.
    pub fn load(
        gl: &Arc<glow::Context>,
        assets: &Arc<Assets>,
        window_size: (u32, u32),
        world_path: PathBuf,
        username: String,
    ) -> Self {
        let server = mp3d_core::server::Server::load(true, world_path.clone())
            .expect("Failed to load world");
        Self::setup(server, gl, assets, window_size, world_path, username)
    }

    fn setup(
        server: mp3d_core::server::Server,
        gl: &Arc<glow::Context>,
        assets: &Arc<Assets>,
        window_size: (u32, u32),
        world_path: PathBuf,
        username: String,
    ) -> Self {
        let connection = LocalConnection::new(server);
        let client = Client::new(connection, username, None);

        let return_to_game = Button::new("Return to Game", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
        let save = Button::new("Save and Quit", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
        let quit = Button::new("Quit", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));

        let layout_ctx = crate::render::ui::widgets::LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        };

        let mut inventory_grid = Grid::new(
            9,
            8.0,
            crate::render::ui::widgets::Alignment::Center,
            Vec4::ZERO,
        );
        for i in 0..36 {
            inventory_grid.add_widget(InventorySlot::new(&client.player.inventory, i));
        }
        let mut inventory_col = Column::new(
            8.0,
            crate::render::ui::widgets::Alignment::Start,
            Vec4::splat(16.0),
            crate::render::ui::widgets::Justification::Start,
            None,
        );
        inventory_col.add_widget(Label::new("Inventory", 36.0, Vec4::ONE));
        inventory_col.add_widget(inventory_grid);
        let mut inventory_stack = Stack::new(
            crate::render::ui::widgets::Alignment::Center,
            crate::render::ui::widgets::Alignment::Center,
            0.0,
        );
        inventory_stack.add_widget(crate::render::ui::widgets::NineSlice::new(
            [UVec2::new(0, 16), UVec2::new(16, 16)],
            inventory_col.size_hint(&layout_ctx),
            UVec4::new(4, 4, 3, 3),
            4,
            0,
            Vec4::ONE,
        ));
        inventory_stack.add_widget(inventory_col);

        let mut hotbar_row = Row::new(
            4.0,
            crate::render::ui::widgets::Alignment::Center,
            Vec4::ZERO,
            crate::render::ui::widgets::Justification::Center,
        );

        for i in 0..9 {
            hotbar_row.add_widget(HotbarSlot::new(&client.player.inventory, i + 3 * 9));
        }

        let mut pause_screen = Column::new(
            20.0,
            crate::render::ui::widgets::Alignment::Center,
            Vec4::ZERO,
            crate::render::ui::widgets::Justification::Center,
            None,
        );
        pause_screen.add_widget(return_to_game);
        pause_screen.add_widget(save);
        pause_screen.add_widget(quit);

        let cloud_renderer = CloudRenderer::new(gl);
        let particle_system = ParticleSystem::new(gl);

        let image_bytes = [
            255, 0, 255, 255, // Pink
            0, 0, 0, 255, // Black
            0, 0, 0, 255, // Black
            255, 0, 255, 255, // Pink
        ];
        let pink_black = Texture::new_bytes(gl, 2, 2, image_bytes.to_vec());

        Self {
            client,
            renderer: WorldRenderer {
                chunk_meshes: HashMap::new(),
                chunk_mesh_pool: Vec::new(),
                cloud_renderer,
                particle_system,
                framebuffer: Framebuffer::new(
                    gl,
                    window_size.0 as i32,
                    window_size.1 as i32,
                    true,
                    &[
                        // Color texture
                        crate::abs::framebuffer::ColorUsage::RGBA8,
                        // Normal texture (unused, might be used in the future)
                        crate::abs::framebuffer::ColorUsage::RGB16F,
                    ],
                ),
                chunk_shader: shader_program!(chunk, gl, ".."),
                entity_shader: shader_program!(entity, gl, ".."),
                postprocess_shader: shader_program!(postprocess, gl, ".."),
                chunk_border_shader: shader_program!(chunk_border, gl, ".."),
                entity_model: crate::render::entities::player_model(gl),
                fullscreen_quad: fullscreen_quad_ndc(gl),
                cube_wireframe: cube_wireframe(gl),
                pink_black,
                profiler: Profiler::new(),
            },
            screen_size: UVec2::new(window_size.0, window_size.1),
            tick_acc: 0.0,
            tick_rate: 48.0,
            ui: SinglePlayerUI {
                chat_input_label: Label::new("", 24.0, Vec4::ONE),
                pause_screen,
                inventory: inventory_stack,
                hotbar: hotbar_row,
                debug_opened: false,
                fps_timer: 0.0,
                fps: 0.0,
                fps_history: [0.0; FPS_HISTORY_LEN],
            },
            world_path,
            mouse_pos: Vec2::ZERO,
            timer: 0.0,
        }
    }

    fn fps_entry(&mut self, fps: f32) {
        self.ui.fps_history.rotate_left(1);
        self.ui.fps_history[FPS_HISTORY_LEN - 1] = fps;
    }

    fn get_recent_messages(&self) -> Vec<TextComponent> {
        self.client
            .messages
            .iter() // All messages
            .rev() // Reversed
            .take(10) // Take the first (last) 10 messages
            .rev() // Reverse back
            .cloned() // Clone the messages so we can own them
            .collect()
    }

    fn draw_chunks(
        &mut self,
        gl: &Arc<glow::Context>,
        assets: &Arc<Assets>,
        view: Mat4,
        projection: Mat4,
    ) {
        let _p = self.renderer.profiler.start_scope("draw_chunks");

        let mut visible: Vec<_> = self.renderer.chunk_meshes.iter().collect();

        visible.sort_by(|(a, _), (b, _)| {
            let da = a.as_vec3() * CHUNK_SIZE as f32 - self.client.player.position;
            let db = b.as_vec3() * CHUNK_SIZE as f32 - self.client.player.position;
            da.length_squared()
                .partial_cmp(&db.length_squared())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let frustum_planes = self.client.player.frustum_planes(
            self.screen_size.x as f32 / self.screen_size.y as f32,
            &self.client.world,
        );

        self.renderer.chunk_shader.use_program();
        self.renderer.chunk_shader.set_uniform("u_view", view);
        self.renderer
            .chunk_shader
            .set_uniform("u_projection", projection);
        self.renderer.chunk_shader.set_uniform("u_texture", 0);
        assets.block_textures.upload(gl).bind(0);
        for (pos, mesh) in visible {
            let [aabb_min, aabb_max] = [
                pos.as_vec3() * CHUNK_SIZE as f32,
                (pos.as_vec3() + Vec3::ONE) * CHUNK_SIZE as f32,
            ];
            if !is_aabb_in_frustum(aabb_min, aabb_max, &frustum_planes) {
                continue;
            }

            mesh.draw();
        }
    }

    fn draw_entities(&mut self, view: Mat4, projection: Mat4, player_model_mat: Mat4) {
        let _p = self.renderer.profiler.start_scope("draw_entities");
        self.renderer.entity_shader.use_program();
        self.renderer
            .entity_shader
            .set_uniform("u_model", player_model_mat);
        self.renderer.entity_shader.set_uniform("u_view", view);
        self.renderer
            .entity_shader
            .set_uniform("u_projection", projection);
        self.renderer.entity_shader.set_uniform("u_texture", 0);
        // TODO: use a proper texture atlas for entities.
        self.renderer.pink_black.bind(0);

        self.renderer.entity_model.draw();
    }

    fn draw_crosshair(ui: &mut UIRenderer, screen_size: Vec2) {
        let center = screen_size / 2.0;

        let hs = CROSSHAIR_SIZE / 2.0;
        let ht = CROSSHAIR_THICKNESS / 2.0;

        let h_rect = [center - Vec2::new(hs, ht), center + Vec2::new(hs, ht)];
        let v_rect = [center - Vec2::new(ht, hs), center + Vec2::new(ht, hs)];

        ui.add_command(DrawCommand::Quad {
            rect: h_rect,
            uv_rect: DEFAULT_UV_RECT,
            mode: UIRenderMode::Color(CROSSHAIR_COLOR),
            layer: 0,
        });

        ui.add_command(DrawCommand::Quad {
            rect: v_rect,
            uv_rect: DEFAULT_UV_RECT,
            mode: UIRenderMode::Color(CROSSHAIR_COLOR),
            layer: 0,
        });
    }

    fn draw_chat(
        &self,
        ui: &mut UIRenderer,
        layout_ctx: &crate::render::ui::widgets::LayoutContext,
        assets: &Assets,
    ) {
        let messages = self.get_recent_messages();
        let message_size = measure_messages(&assets.font, &messages, 24.0);

        let hotbar_size = self.ui.hotbar.size_hint(layout_ctx);

        let mut messages_start_y =
            self.screen_size.y as f32 - message_size.y - 10.0 - hotbar_size.y - 15.0;

        if self.client.gui.chat().is_some() {
            messages_start_y -= 24.0 + 10.0;
            let label_size = self.ui.chat_input_label.size_hint(layout_ctx);
            ui.add_command(DrawCommand::Quad {
                rect: [
                    Vec2::new(
                        5.0,
                        self.screen_size.y as f32 - label_size.y - 15.0 - hotbar_size.y - 10.0,
                    ),
                    Vec2::new(
                        5.0 + label_size.x + 10.0,
                        self.screen_size.y as f32 - 5.0 - hotbar_size.y - 15.0,
                    ),
                ],
                uv_rect: DEFAULT_UV_RECT,
                mode: UIRenderMode::Color(Vec4::new(0.0, 0.0, 0.0, 0.5)),
                layer: 0,
            });
            self.ui.chat_input_label.draw(ui, assets);
        }

        ui.add_command(crate::render::ui::uirenderer::DrawCommand::Quad {
            rect: [
                Vec2::new(5.0, messages_start_y - 5.0),
                Vec2::new(
                    5.0 + message_size.x + 10.0,
                    messages_start_y + message_size.y + 5.0,
                ),
            ],
            uv_rect: DEFAULT_UV_RECT,
            mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(0.0, 0.0, 0.0, 0.5)),
            layer: 0,
        });
        for cmd in text_messages(
            &assets.font,
            &messages,
            24.0,
            Vec2::new(10.0, messages_start_y),
        ) {
            ui.add_command(cmd);
        }
        ui.finish();
    }
}

impl super::Scene for SinglePlayer {
    fn handle_event(&mut self, gl: &Arc<glow::Context>, event: &sdl2::event::Event) {
        if let sdl2::event::Event::Window {
            win_event: sdl2::event::WindowEvent::Resized(width, height),
            ..
        } = event
        {
            self.screen_size.x = *width as u32;
            self.screen_size.y = *height as u32;
            unsafe {
                gl.viewport(0, 0, *width, *height);
            }
            self.renderer.framebuffer.resize(*width, *height);
        }
    }

    fn update(&mut self, ctx: &mut SceneUpdateContext) -> super::SceneAction {
        let SceneUpdateContext {
            gl,
            ctx,
            window,
            sdl_ctx,
            assets,
            config,
            ..
        } = ctx;

        self.renderer.profiler.begin_frame();

        let layout_ctx = crate::render::ui::widgets::LayoutContext {
            max_size: Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
            cursor: Vec2::ZERO,
            assets,
        };

        window.set_title("Mineplace3D - Single Player").unwrap();
        sdl_ctx
            .mouse()
            .set_relative_mouse_mode(self.client.gui.none());
        self.timer += ctx.delta_time;
        self.ui.fps_timer += ctx.delta_time;

        let fps = 1.0 / ctx.delta_time;
        self.fps_entry(fps);
        if self.ui.fps_timer > 0.5 {
            self.ui.fps = fps;
            self.ui.fps_timer = 0.0;
        }

        if ctx.keyboard.pressed.contains(&sdl2::keyboard::Keycode::F6) {
            return super::SceneAction::ReloadAssets;
        }

        {
            let _p = self.renderer.profiler.start_scope("client_update");

            self.client
                .send_input(ctx, ctx.delta_time, config.read().unwrap().sensitivity());

            if !self.client.gui.pause_menu() {
                if ctx.keyboard.pressed.contains(&sdl2::keyboard::Keycode::F3) {
                    self.ui.debug_opened = !self.ui.debug_opened;
                }

                if let Err(_reason) = self
                    .client
                    .receive_state(&mut self.renderer.particle_system)
                {
                    todo!("Save world and exit.")
                }
            } else {
                self.ui.pause_screen.update(ctx);
                self.ui
                    .pause_screen
                    .layout(&crate::render::ui::widgets::LayoutContext {
                        max_size: Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
                        cursor: Vec2::ZERO,
                        assets,
                    });
                if self
                    .ui
                    .pause_screen
                    .get_widget::<Button>(0)
                    .is_some_and(|btn| btn.is_released())
                {
                    self.client.gui = CurrentGUI::None;
                }
                if self
                    .ui
                    .pause_screen
                    .get_widget::<Button>(1)
                    .is_some_and(|btn| btn.is_released())
                {
                    log::info!("Saving world...");
                    std::fs::create_dir_all(&self.world_path)
                        .expect("Failed to create world directory");
                    self.client
                        .connection
                        .server
                        .save()
                        .expect("Failed to save world");

                    return super::SceneAction::Pop;
                }
                if self
                    .ui
                    .pause_screen
                    .get_widget::<Button>(2)
                    .is_some_and(|btn| btn.is_released())
                {
                    return super::SceneAction::Pop;
                }
            }
        }

        {
            let tick_time = 1.0f32 / self.tick_rate;
            self.tick_acc = (self.tick_acc + ctx.delta_time).min(tick_time * 5.0);

            let _p = self.renderer.profiler.start_scope("server_update");

            while self.tick_acc >= tick_time {
                self.client.connection.tick(self.tick_rate as u8);
                self.tick_acc -= tick_time;
            }
        }

        let hotbar_size = self.ui.hotbar.size_hint(&layout_ctx);

        if let Some(chat) = self.client.gui.chat() {
            self.ui.chat_input_label.text = chat.clone();
        } else {
            self.ui.chat_input_label.text = "".to_string();
        }
        self.ui.chat_input_label.update(ctx);
        self.ui
            .chat_input_label
            .layout(&crate::render::ui::widgets::LayoutContext {
                max_size: Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
                cursor: Vec2::new(
                    10.0,
                    self.screen_size.y as f32 - 24.0 - 10.0 - hotbar_size.y - 15.0,
                ),
                assets,
            });
        if self.client.gui.inventory() {
            self.ui.inventory.update(ctx);
            let inventory_size = self.ui.inventory.size_hint(&layout_ctx);
            self.ui
                .inventory
                .layout(&crate::render::ui::widgets::LayoutContext {
                    max_size: inventory_size,
                    cursor: self.screen_size.as_vec2() / 2.0 - inventory_size / 2.0,
                    assets,
                });
        }
        self.ui.hotbar.update(ctx);
        let hotbar_size = self.ui.hotbar.size_hint(&layout_ctx);
        self.ui
            .hotbar
            .layout(&crate::render::ui::widgets::LayoutContext {
                max_size: hotbar_size,
                cursor: Vec2::new(
                    self.screen_size.x as f32 / 2.0 - hotbar_size.x / 2.0,
                    self.screen_size.y as f32 - hotbar_size.y - 10.0,
                ),
                assets,
            });
        let unloaded = self.client.world.unload_chunks(self.client.player.position);
        for pos in unloaded {
            if let Some(mesh) = self.renderer.chunk_meshes.remove(&pos) {
                self.renderer.chunk_mesh_pool.push(mesh);
            }
        }
        {
            let _p = self.renderer.profiler.start_scope("particles");
            self.renderer.particle_system.update(ctx.delta_time, assets);
        }
        {
            let _p = self.renderer.profiler.start_scope("world_meshing");
            if !self.client.world.remesh_queue.is_empty() {
                mesh_world(
                    gl,
                    &mut self.client.world,
                    &mut self.renderer.chunk_meshes,
                    &mut self.renderer.chunk_mesh_pool,
                    &assets.block_textures,
                    &assets.block_models,
                );
            }
        }
        self.mouse_pos = ctx.mouse.position;

        super::SceneAction::None
    }

    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut UIRenderer,
        assets: &Arc<Assets>,
        _config: &Arc<RwLock<super::options::ClientConfig>>,
    ) {
        let layout_ctx = crate::render::ui::widgets::LayoutContext {
            max_size: Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
            cursor: Vec2::ZERO,
            assets,
        };

        let player_model_mat = self.client.player.model();
        let view = self.client.player.view(&self.client.world);
        let projection = self
            .client
            .player
            .projection(self.screen_size.x as f32 / self.screen_size.y as f32);

        unsafe {
            // SETUP

            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.clear_color(0.7, 0.7, 0.9, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            // WORLD

            {
                let _fb = self.renderer.framebuffer.guard();

                gl.clear_color(0.7, 0.7, 0.9, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

                // CHUNKS

                self.draw_chunks(gl, assets, view, projection);

                // PLAYER

                self.draw_entities(view, projection, player_model_mat);

                // PARTICLES

                {
                    let _p = self.renderer.profiler.start_scope("particles");
                    self.renderer
                        .particle_system
                        .render(gl, assets, view, projection);
                }

                // CLOUDS

                self.renderer.cloud_renderer.draw(
                    gl,
                    projection,
                    view,
                    self.client.player.position,
                    self.timer,
                );

                // DEBUG - CHUNK BORDERS

                if self.ui.debug_opened {
                    self.renderer.chunk_border_shader.use_program();
                    self.renderer
                        .chunk_border_shader
                        .set_uniform("u_view", view);
                    self.renderer
                        .chunk_border_shader
                        .set_uniform("u_projection", projection);

                    for pos in self.renderer.chunk_meshes.keys() {
                        let world_pos = pos.as_vec3() * CHUNK_SIZE as f32;

                        self.renderer
                            .chunk_border_shader
                            .set_uniform("u_offset", world_pos);
                        self.renderer
                            .chunk_border_shader
                            .set_uniform("u_scale", CHUNK_SIZE as f32);

                        self.renderer.cube_wireframe.draw();
                    }
                }
            }

            // POSTPROCESS

            gl.disable(glow::CULL_FACE);
            gl.depth_mask(false);

            self.renderer.postprocess_shader.use_program();
            self.renderer.postprocess_shader.set_uniform("u_texture", 0);
            self.renderer
                .postprocess_shader
                .set_uniform("u_time", self.timer);
            self.renderer.framebuffer.textures()[0].bind(0);
            self.renderer.fullscreen_quad.draw();

            // UI

            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::DEPTH_TEST);

            // CROSSHAIR

            Self::draw_crosshair(ui, self.screen_size.as_vec2());

            // CHAT MESSAGES

            self.draw_chat(ui, &layout_ctx, assets);

            // DEBUG - TEXT & GRAPHS

            if self.ui.debug_opened {
                let block_pos = self.client.player.position.as_ivec3();
                let chunk = block_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
                let chunk_local = block_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

                let text = format!(
                    r#"Mineplace3D v{}

{} FPS

X: {:.2} Y: {:.2} Z: {:.2}
Yaw: {:.2} Pitch: {:.2}

Block: X: {} Y: {} Z: {}
Chunk: X: {} Y: {} Z: {}
Chunk local: X: {} Y: {} Z: {}"#,
                    env!("CARGO_PKG_VERSION"),
                    self.ui.fps as u32,
                    self.client.player.position.x,
                    self.client.player.position.y,
                    self.client.player.position.z,
                    self.client.player.yaw,
                    self.client.player.pitch,
                    block_pos.x,
                    block_pos.y,
                    block_pos.z,
                    chunk.x,
                    chunk.y,
                    chunk.z,
                    chunk_local.x,
                    chunk_local.y,
                    chunk_local.z,
                );

                for mut cmd in assets.font.text(&text, TextParams::default()) {
                    match &mut cmd {
                        DrawCommand::Quad { rect, .. } => {
                            rect[0] += Vec2::new(10.0, 10.0);
                            rect[1] += Vec2::new(10.0, 10.0);
                        }
                        DrawCommand::Mesh { vertices, .. } => {
                            for v in vertices {
                                v.position += Vec3::new(10.0, 10.0, 0.0);
                            }
                        }
                    }
                    ui.add_command(cmd);
                }

                // draw the fps graph on the top right side and also show the current, average, min
                // and max fps
                let graph_x = self.screen_size.x as f32 - FPS_GRAPH_WIDTH - 10.0;
                let bar_width = FPS_GRAPH_WIDTH / FPS_HISTORY_LEN as f32;
                let max_fps = self.ui.fps_history.iter().cloned().fold(f32::NAN, f32::max);
                let min_fps = self.ui.fps_history.iter().cloned().fold(f32::NAN, f32::min);
                let average_fps = self.ui.fps_history.iter().sum::<f32>() / FPS_HISTORY_LEN as f32;
                for (i, fps) in self.ui.fps_history.iter().enumerate() {
                    let x = graph_x + i as f32 / FPS_HISTORY_LEN as f32 * FPS_GRAPH_WIDTH;
                    let y = FPS_GRAPH_Y + FPS_GRAPH_HEIGHT - (fps / max_fps * FPS_GRAPH_HEIGHT);
                    let bar_height = FPS_GRAPH_Y + FPS_GRAPH_HEIGHT - y;
                    ui.add_command(DrawCommand::Quad {
                        rect: [Vec2::new(x, y), Vec2::new(x + bar_width, y + bar_height)],
                        uv_rect: DEFAULT_UV_RECT,
                        mode: UIRenderMode::Color(Vec4::new(0.0, 1.0, 0.0, 0.6)),
                        layer: 0,
                    });
                }

                let stats_text = format!(
                    "FPS: {:.2}\nAvg: {:.2}\nMin: {:.2}\nMax: {:.2}",
                    self.ui.fps, average_fps, min_fps, max_fps
                );
                let measurement = assets
                    .font
                    .measure_text(&stats_text, ColorlessTextParams::default());
                let text_x = self.screen_size.x as f32 - measurement.x - 10.0;
                let text_y = FPS_GRAPH_Y + FPS_GRAPH_HEIGHT + 10.0;
                for mut cmd in assets.font.text(&stats_text, TextParams::default()) {
                    match &mut cmd {
                        DrawCommand::Quad { rect, .. } => {
                            rect[0] += Vec2::new(text_x, text_y);
                            rect[1] += Vec2::new(text_x, text_y);
                        }
                        DrawCommand::Mesh { vertices, .. } => {
                            for v in vertices {
                                v.position += Vec3::new(text_x, text_y, 0.0);
                            }
                        }
                    }
                    ui.add_command(cmd);
                }

                // profiler horizontal bar graph
                let total_time: f32 = self
                    .renderer
                    .profiler
                    .smoothed_entries
                    .iter()
                    .map(|entry| entry.duration.as_secs_f32() * 1000.0)
                    .sum();
                let graph_height = 35.0 * self.renderer.profiler.entries.len() as f32;
                let graph_x = self.screen_size.x as f32 - PROFILER_GRAPH_WIDTH - 10.0;
                let mut current_y = self.screen_size.y as f32 - graph_height - 10.0;
                for entry in self.renderer.profiler.smoothed_entries.iter() {
                    let entry_time = entry.duration.as_secs_f32() * 1000.0;
                    let bar_width = entry_time / total_time * PROFILER_GRAPH_WIDTH;
                    ui.add_command(DrawCommand::Quad {
                        rect: [
                            Vec2::new(graph_x, current_y),
                            Vec2::new(graph_x + bar_width, current_y + 30.0),
                        ],
                        uv_rect: DEFAULT_UV_RECT,
                        mode: UIRenderMode::Color(Vec4::new(0.0, 0.0, 1.0, 0.6)),
                        layer: 0,
                    });
                    let entry_text = format!("{}: {:.2} ms", entry.name, entry_time);
                    let text_x = graph_x + 5.0;
                    let text_y = current_y + 1.0;
                    for mut cmd in assets.font.text(&entry_text, TextParams::default()) {
                        match &mut cmd {
                            DrawCommand::Quad { rect, .. } => {
                                rect[0] += Vec2::new(text_x, text_y);
                                rect[1] += Vec2::new(text_x, text_y);
                            }
                            DrawCommand::Mesh { vertices, .. } => {
                                for v in vertices {
                                    v.position += Vec3::new(text_x, text_y, 0.0);
                                }
                            }
                        }
                        ui.add_command(cmd);
                    }
                    current_y += 35.0;
                }
            }

            // PAUSE MENU

            if self.client.gui.pause_menu() {
                ui.add_command(DrawCommand::Quad {
                    rect: [Vec2::ZERO, self.screen_size.as_vec2()],
                    uv_rect: DEFAULT_UV_RECT,
                    mode: UIRenderMode::Color(Vec4::new(0.0, 0.0, 0.0, 0.5)),
                    layer: -1,
                });

                self.ui.pause_screen.draw(ui, assets);
            }

            // INVENTORY & HOTBAR

            if self.client.gui.inventory() {
                self.ui.inventory.draw(ui, assets);

                let temp_stack = &self.client.player.inventory.borrow().inner.temp;
                if !temp_stack.is_empty() {
                    // Draw the temp stack at the mouse position
                    let temp_stack_commands = InventorySlot::draw_stack(
                        *temp_stack,
                        assets,
                        self.mouse_pos,
                        ui,
                        &assets.font,
                    );
                    for cmd in temp_stack_commands {
                        ui.add_command(cmd);
                    }
                }
            }
            self.ui.hotbar.draw(ui, assets);
        }

        self.renderer.profiler.end_frame();
    }
}

fn measure_messages(font: &Font, messages: &[TextComponent], font_size: f32) -> Vec2 {
    let mut size = Vec2::ZERO;
    for message in messages {
        let message_size = font.measure_component(
            message,
            ColorlessTextParams {
                font_size,
                word_wrap_width: Some(400.0),
            },
        );
        size.x = size.x.max(message_size.x);
        size.y += message_size.y;
    }
    size
}

fn text_messages(
    font: &Font,
    messages: &[TextComponent],
    font_size: f32,
    pos: Vec2,
) -> Vec<DrawCommand> {
    let mut commands = Vec::new();
    let mut cursor = pos;
    for message in messages {
        let message_commands = font.text_component(
            message,
            ColorlessTextParams {
                font_size,
                word_wrap_width: Some(400.0),
            },
        );
        for mut cmd in message_commands {
            if let DrawCommand::Quad { rect, .. } = &mut cmd {
                rect[0] += cursor;
                rect[1] += cursor;
            } else if let DrawCommand::Mesh { vertices, .. } = &mut cmd {
                for vertex in vertices {
                    vertex.position += cursor.extend(0.0);
                }
            }
            commands.push(cmd);
        }
        let message_size = font.measure_component(
            message,
            ColorlessTextParams {
                font_size,
                word_wrap_width: Some(400.0),
            },
        );
        cursor.y += message_size.y;
    }
    commands
}

fn is_aabb_in_frustum(aabb_min: Vec3, aabb_max: Vec3, planes: &[Vec4; 6]) -> bool {
    for plane in planes {
        let p = Vec3::new(
            if plane.x >= 0.0 {
                aabb_max.x
            } else {
                aabb_min.x
            },
            if plane.y >= 0.0 {
                aabb_max.y
            } else {
                aabb_min.y
            },
            if plane.z >= 0.0 {
                aabb_max.z
            } else {
                aabb_min.z
            },
        );
        if plane.truncate().dot(p) + plane.w < 0.0 {
            return false;
        }
    }
    true
}

fn fullscreen_quad_ndc(gl: &Arc<glow::Context>) -> Mesh {
    Mesh::new(
        gl,
        &[
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ],
        &[0, 1, 2, 2, 3, 0],
        glow::TRIANGLES,
    )
}

fn cube_wireframe(gl: &Arc<glow::Context>) -> Mesh {
    let vertices = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    ];

    let indices = [
        0, 1, 1, 2, 2, 3, 3, 0, // bottom
        4, 5, 5, 6, 6, 7, 7, 4, // top
        0, 4, 1, 5, 2, 6, 3, 7, // verticals
    ];

    Mesh::new(gl, &vertices, &indices, glow::LINES)
}
