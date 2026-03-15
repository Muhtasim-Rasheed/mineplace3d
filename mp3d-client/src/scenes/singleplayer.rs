//! The single player scene implementation.

use std::{
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, RwLock},
};

use glam::{IVec3, UVec2, UVec4, Vec2, Vec3, Vec4};
use glow::HasContext;
use mp3d_core::{TextComponent, world::chunk::CHUNK_SIZE};

use crate::{
    abs::{Mesh, ShaderProgram, Texture, TextureHandle, framebuffer::Framebuffer},
    client::{Client, Connection, LocalConnection},
    render::{clouds::CloudRenderer, meshing::mesh_world, ui::widgets::*},
    shader_program,
};

struct SinglePlayerUI {
    chat_input_label: Option<Label>,
    pause_screen: Column,
    inventory: Stack,
    font: Rc<Font>,
}

struct WorldRenderer {
    chunk_meshes: HashMap<IVec3, Mesh>,
    chunk_mesh_pool: Vec<Mesh>,
    cloud_renderer: CloudRenderer,
    framebuffer: Framebuffer,
    ssao_framebuffer: Framebuffer,
    chunk_shader: ShaderProgram,
    ssao_shader: ShaderProgram,
    postprocess_shader: ShaderProgram,
    ssao_kernel: [Vec3; 32],
    ssao_noise_texture: Texture,

    fullscreen_quad: Mesh,
}

/// The [`SinglePlayer`] struct represents the single player scene.
pub struct SinglePlayer {
    client: Client<LocalConnection>,
    renderer: WorldRenderer,
    screen_size: UVec2,
    tick_acc: f32,
    tick_rate: f32,
    playing: bool,
    ui: SinglePlayerUI,
    world_path: PathBuf,
    mouse_pos: Vec2,
    total_time: f32,
}

impl SinglePlayer {
    /// Creates a new [`SinglePlayer`] instance.
    pub fn new(
        gl: &Arc<glow::Context>,
        font: &Rc<Font>,
        gui_tex: TextureHandle,
        window_size: (u32, u32),
        seed: i32,
        world_path: PathBuf,
        username: String,
    ) -> Self {
        let server = mp3d_core::server::Server::new(true, seed, world_path.clone());
        Self::setup(server, gl, font, gui_tex, window_size, world_path, username)
    }

    /// Loads a world from the given path and creates a new [`SinglePlayer`] instance.
    pub fn load(
        gl: &Arc<glow::Context>,
        font: &Rc<Font>,
        gui_tex: TextureHandle,
        window_size: (u32, u32),
        world_path: PathBuf,
        username: String,
    ) -> Self {
        let server = mp3d_core::server::Server::load(true, world_path.clone())
            .expect("Failed to load world");
        Self::setup(server, gl, font, gui_tex, window_size, world_path, username)
    }

    fn setup(
        server: mp3d_core::server::Server,
        gl: &Arc<glow::Context>,
        font: &Rc<Font>,
        gui_tex: TextureHandle,
        window_size: (u32, u32),
        world_path: PathBuf,
        username: String,
    ) -> Self {
        let connection = LocalConnection::new(server);
        let client = Client::new(connection, username, None);
        let chunk_shader = shader_program!(chunk, gl, "..");
        let ssao_shader = shader_program!(ssao, gl, "..");
        let postprocess_shader = shader_program!(postprocess, gl, "..");

        let mut ssao_kernel = [Vec3::ZERO; 32];
        for (i, sample) in ssao_kernel.iter_mut().enumerate() {
            *sample = Vec3::new(
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>(),
            );
            *sample = sample.normalize() * rand::random::<f32>();
            let scale = i as f32 / 32.0;
            let scale = 0.1 + 0.9 * scale * scale;
            *sample *= scale;
        }

        let mut data = vec![0u8; 16 * 16 * 4];
        for i in 0..16 * 16 {
            let noise = Vec3::new(
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>() * 2.0 - 1.0,
                0.0,
            )
            .normalize();
            data[i * 4] = ((noise.x * 0.5 + 0.5) * 255.0) as u8;
            data[i * 4 + 1] = ((noise.y * 0.5 + 0.5) * 255.0) as u8;
            data[i * 4 + 2] = ((noise.z * 0.5 + 0.5) * 255.0) as u8;
            data[i * 4 + 3] = 255;
        }
        let ssao_noise_texture = Texture::new_bytes(gl, 16, 16, data);

        let return_to_game = Button::new(
            "Return to Game",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );
        let save = Button::new(
            "Save and Quit",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );
        let quit = Button::new(
            "Quit",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );

        let mut inventory_grid = Grid::new(
            9,
            8.0,
            crate::render::ui::widgets::Alignment::Center,
            Vec4::ZERO,
        );
        for i in 0..36 {
            inventory_grid.add_widget(InventorySlot::new(
                gui_tex,
                font,
                &client.player.inventory,
                i,
            ));
        }
        let mut inventory_col = Column::new(
            8.0,
            crate::render::ui::widgets::Alignment::Start,
            Vec4::splat(16.0),
            crate::render::ui::widgets::Justification::Start,
            None,
        );
        inventory_col.add_widget(Label::new("Inventory", 36.0, Vec4::ONE, font));
        inventory_col.add_widget(inventory_grid);
        let mut inventory_stack = Stack::new(
            crate::render::ui::widgets::Alignment::Center,
            crate::render::ui::widgets::Alignment::Center,
            0.0,
        );
        inventory_stack.add_widget(crate::render::ui::widgets::NineSlice::new(
            gui_tex,
            [UVec2::new(0, 16), UVec2::new(16, 16)],
            inventory_col.size_hint(),
            UVec4::new(4, 4, 3, 3),
            4,
            0,
            Vec4::ONE,
        ));
        inventory_stack.add_widget(inventory_col);

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

        Self {
            client,
            renderer: WorldRenderer {
                chunk_meshes: HashMap::new(),
                chunk_mesh_pool: Vec::new(),
                cloud_renderer,
                framebuffer: Framebuffer::new(
                    gl,
                    window_size.0 as i32,
                    window_size.1 as i32,
                    true,
                    &[
                        // Color texture
                        crate::abs::framebuffer::ColorUsage::RGBA8,
                        // Normal texture
                        crate::abs::framebuffer::ColorUsage::RGB16F,
                    ],
                ),
                ssao_framebuffer: Framebuffer::new(
                    gl,
                    window_size.0 as i32 / 2,
                    window_size.1 as i32 / 2,
                    false,
                    &[crate::abs::framebuffer::ColorUsage::R32F],
                ),
                chunk_shader,
                ssao_shader,
                postprocess_shader,
                ssao_kernel,
                ssao_noise_texture,
                fullscreen_quad: fullscreen_quad_ndc(gl),
            },
            screen_size: UVec2::new(window_size.0, window_size.1),
            tick_acc: 0.0,
            tick_rate: 48.0,
            playing: true,
            ui: SinglePlayerUI {
                chat_input_label: None,
                pause_screen,
                inventory: inventory_stack,
                font: font.clone(),
            },
            world_path,
            mouse_pos: Vec2::ZERO,
            total_time: 0.0,
        }
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
            self.renderer
                .ssao_framebuffer
                .resize(*width / 2, *height / 2);
        }
        if let sdl2::event::Event::KeyDown { keycode, .. } = event
            && *keycode == Some(sdl2::keyboard::Keycode::Escape)
            && !self.client.chat_open
            && !self.client.inventory_open
        {
            self.playing = !self.playing;
        }
    }

    fn update(
        &mut self,
        gl: &Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        window: &mut sdl2::video::Window,
        sdl_ctx: &sdl2::Sdl,
        assets: &Arc<super::Assets>,
        _config: &Arc<RwLock<super::options::ClientConfig>>,
    ) -> super::SceneSwitch {
        window.set_title("Mineplace3D - Single Player").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(
            self.playing && !self.client.chat_open && !self.client.inventory_open,
        );
        self.total_time += ctx.delta_time;
        if self.playing {
            self.client.send_input(ctx, ctx.delta_time);
            if let Err(_reason) = self.client.recieve_state() {
                todo!("Save world and exit.")
            }
        } else {
            self.client.inventory_open = false;
            self.client.chat_open = false;
            self.ui.pause_screen.update(ctx);
            self.ui
                .pause_screen
                .layout(&crate::render::ui::widgets::LayoutContext {
                    max_size: Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
                    cursor: Vec2::ZERO,
                });
            if self
                .ui
                .pause_screen
                .get_widget::<Button>(0)
                .is_some_and(|btn| btn.is_released())
            {
                self.playing = true;
            }
            if self
                .ui
                .pause_screen
                .get_widget::<Button>(1)
                .is_some_and(|btn| btn.is_released())
            {
                std::fs::create_dir_all(&self.world_path)
                    .expect("Failed to create world directory");
                self.client
                    .connection
                    .server
                    .save()
                    .expect("Failed to save world");

                return super::SceneSwitch::Pop;
            }
            if self
                .ui
                .pause_screen
                .get_widget::<Button>(2)
                .is_some_and(|btn| btn.is_released())
            {
                return super::SceneSwitch::Pop;
            }
        }
        let tick_time = 1.0 / self.tick_rate;
        self.tick_acc += ctx.delta_time;
        if self.tick_acc > tick_time * 5.0 {
            self.tick_acc = tick_time * 5.0;
        }
        while self.tick_acc >= tick_time {
            self.client.connection.tick(self.tick_rate as u8);
            self.tick_acc -= tick_time;
        }
        if let Some(chat) = self.client.chat_message.as_ref() {
            if let Some(label) = self.ui.chat_input_label.as_mut() {
                label.text = chat.clone();
            } else {
                self.ui.chat_input_label = Some(Label::new(chat, 24.0, Vec4::ONE, &self.ui.font));
            }
        } else {
            self.ui.chat_input_label = None;
        }
        if let Some(label) = self.ui.chat_input_label.as_mut() {
            label.update(ctx);
            label.layout(&crate::render::ui::widgets::LayoutContext {
                max_size: Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
                cursor: Vec2::new(10.0, self.screen_size.y as f32 - 34.0),
            });
        }
        if self.client.inventory_open {
            self.ui.inventory.update(ctx);
            let inventory_size = self.ui.inventory.size_hint();
            self.ui
                .inventory
                .layout(&crate::render::ui::widgets::LayoutContext {
                    max_size: inventory_size,
                    cursor: Vec2::new(
                        self.screen_size.x as f32 / 2.0 - inventory_size.x / 2.0,
                        self.screen_size.y as f32 / 2.0 - inventory_size.y / 2.0,
                    ),
                });
        }
        let unloaded = self
            .client
            .world
            .unload_chunks(self.client.player.position.as_ivec3());
        for pos in unloaded {
            if let Some(mesh) = self.renderer.chunk_meshes.remove(&pos) {
                self.renderer.chunk_mesh_pool.push(mesh);
            }
        }
        if !self.client.world.remesh_queue.is_empty() {
            mesh_world(
                gl,
                &mut self.client.world,
                &mut self.renderer.chunk_meshes,
                &mut self.renderer.chunk_mesh_pool,
                &assets.block_textures,
                &assets.block_models,
                self.client
                    .player
                    .position
                    .as_ivec3()
                    .div_euclid(IVec3::splat(CHUNK_SIZE as i32)),
            );
        }
        self.mouse_pos = ctx.mouse.position;
        super::SceneSwitch::None
    }

    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut crate::render::ui::uirenderer::UIRenderer,
        assets: &Arc<super::Assets>,
        _config: &Arc<RwLock<super::options::ClientConfig>>,
    ) {
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.clear_color(0.7, 0.7, 0.9, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.renderer.framebuffer.bind();

            gl.clear_color(0.7, 0.7, 0.9, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.renderer.chunk_shader.use_program();
            self.renderer
                .chunk_shader
                .set_uniform("u_view", self.client.player.view());
            self.renderer.chunk_shader.set_uniform(
                "u_projection",
                self.client
                    .player
                    .projection(self.screen_size.x as f32 / self.screen_size.y as f32),
            );
            self.renderer.chunk_shader.set_uniform("u_texture", 0);
            assets.block_textures.upload(gl).bind(0);
            for (pos, mesh) in &self.renderer.chunk_meshes {
                let [aabb_min, aabb_max] = [
                    pos.as_vec3() * CHUNK_SIZE as f32,
                    (pos.as_vec3() + Vec3::splat(1.0)) * CHUNK_SIZE as f32,
                ];
                if !is_aabb_in_frustum(
                    aabb_min,
                    aabb_max,
                    &self
                        .client
                        .player
                        .frustum_planes(self.screen_size.x as f32 / self.screen_size.y as f32),
                ) {
                    continue;
                }

                mesh.draw();
            }

            gl.disable(glow::CULL_FACE);
            gl.depth_mask(false);
            self.renderer.cloud_renderer.shader.use_program();
            self.renderer
                .cloud_renderer
                .shader
                .set_uniform("u_view", self.client.player.view());
            self.renderer.cloud_renderer.shader.set_uniform(
                "u_projection",
                self.client
                    .player
                    .projection(self.screen_size.x as f32 / self.screen_size.y as f32),
            );
            self.renderer
                .cloud_renderer
                .shader
                .set_uniform("u_camera_pos", self.client.player.position);
            self.renderer
                .cloud_renderer
                .shader
                .set_uniform("u_time", self.total_time);
            self.renderer
                .cloud_renderer
                .shader
                .set_uniform("u_texture", 0);
            self.renderer.cloud_renderer.texture.bind(0);
            self.renderer.cloud_renderer.mesh.draw();

            Framebuffer::unbind(gl, self.screen_size.x as i32, self.screen_size.y as i32);

            gl.disable(glow::CULL_FACE);

            self.renderer.ssao_framebuffer.bind();
            gl.clear_color(1.0, 1.0, 1.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            self.renderer.ssao_shader.use_program();
            self.renderer.ssao_shader.set_uniform("u_depth", 0);
            self.renderer.ssao_shader.set_uniform("u_normal", 1);
            self.renderer.ssao_shader.set_uniform("u_noise", 2);
            self.renderer.ssao_shader.set_uniform(
                "u_noise_scale",
                Vec2::new(
                    self.screen_size.x as f32 / 16.0,
                    self.screen_size.y as f32 / 16.0,
                ),
            );
            self.renderer
                .ssao_shader
                .set_uniform("u_samples", self.renderer.ssao_kernel);
            self.renderer.ssao_shader.set_uniform(
                "u_projection",
                self.client
                    .player
                    .projection(self.screen_size.x as f32 / self.screen_size.y as f32),
            );
            self.renderer.ssao_shader.set_uniform(
                "u_inv_projection",
                self.client
                    .player
                    .projection(self.screen_size.x as f32 / self.screen_size.y as f32)
                    .inverse(),
            );
            self.renderer.ssao_shader.set_uniform(
                "u_view_normal",
                glam::Mat3::from_mat4(self.client.player.view())
                    .inverse()
                    .transpose(),
            );
            self.renderer.framebuffer.depth_texture().unwrap().bind(0);
            self.renderer.framebuffer.textures()[1].bind(1);
            self.renderer.ssao_noise_texture.bind(2);
            self.renderer.fullscreen_quad.draw();

            Framebuffer::unbind(gl, self.screen_size.x as i32, self.screen_size.y as i32);
            gl.depth_mask(false);

            self.renderer.postprocess_shader.use_program();
            self.renderer.postprocess_shader.set_uniform("u_texture", 0);
            self.renderer.postprocess_shader.set_uniform("u_depth", 1);
            self.renderer.postprocess_shader.set_uniform("u_ssao", 2);
            self.renderer
                .postprocess_shader
                .set_uniform("u_time", self.total_time);
            self.renderer.framebuffer.textures()[0].bind(0);
            self.renderer.framebuffer.depth_texture().unwrap().bind(1);
            self.renderer.ssao_framebuffer.textures()[0].bind(2);
            self.renderer.fullscreen_quad.draw();

            gl.clear(glow::DEPTH_BUFFER_BIT);

            // draw chat messages
            let messages = self
                .client
                .messages
                .iter()
                .rev()
                .take(10)
                .rev()
                .cloned()
                .collect::<Vec<_>>();
            let message_size = measure_messages(&self.ui.font, &messages, 24.0);

            let mut messages_start_y = self.screen_size.y as f32 - message_size.y - 10.0;

            if let Some(chat) = self.ui.chat_input_label.as_ref() {
                messages_start_y -= 34.0 + 15.0;
                let label_size = chat.size_hint();
                ui.add_command(crate::render::ui::uirenderer::DrawCommand::Quad {
                    rect: [
                        Vec2::new(5.0, self.screen_size.y as f32 - label_size.y - 15.0),
                        Vec2::new(5.0 + label_size.x + 10.0, self.screen_size.y as f32 - 5.0),
                    ],
                    uv_rect: [Vec2::ZERO, Vec2::ONE],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(
                        0.0, 0.0, 0.0, 0.5,
                    )),
                    layer: 0,
                });
                ui.finish();
                chat.draw(ui, assets);
            }

            ui.add_command(crate::render::ui::uirenderer::DrawCommand::Quad {
                rect: [
                    Vec2::new(5.0, messages_start_y - 5.0),
                    Vec2::new(
                        5.0 + message_size.x + 10.0,
                        messages_start_y + message_size.y + 5.0,
                    ),
                ],
                uv_rect: [Vec2::ZERO, Vec2::ONE],
                mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(
                    0.0, 0.0, 0.0, 0.5,
                )),
                layer: 0,
            });
            for cmd in text_messages(
                &self.ui.font,
                &messages,
                24.0,
                Vec2::new(10.0, messages_start_y),
            ) {
                ui.add_command(cmd);
            }
            ui.finish();

            if !self.playing {
                ui.add_command(crate::render::ui::uirenderer::DrawCommand::Quad {
                    rect: [
                        Vec2::new(0.0, 0.0),
                        Vec2::new(self.screen_size.x as f32, self.screen_size.y as f32),
                    ],
                    uv_rect: [Vec2::ZERO, Vec2::ONE],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(
                        0.0, 0.0, 0.0, 0.5,
                    )),
                    layer: -1,
                });
                ui.finish();

                self.ui.pause_screen.draw(ui, assets);
            }

            if self.client.inventory_open {
                self.ui.inventory.draw(ui, assets);

                let temp_stack = &self.client.player.inventory.borrow().inner.temp;
                if !temp_stack.is_empty() {
                    // Draw the temp stack at the mouse position
                    let temp_stack_commands = InventorySlot::draw_stack(
                        *temp_stack,
                        assets,
                        self.mouse_pos,
                        ui,
                        &self.ui.font,
                    );
                    for cmd in temp_stack_commands {
                        ui.add_command(cmd);
                    }
                }
            }
        }
    }
}

fn measure_messages(font: &Font, messages: &[TextComponent], font_size: f32) -> Vec2 {
    let mut size = Vec2::ZERO;
    for message in messages {
        let message_size = font.measure_component(message, font_size);
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
) -> Vec<crate::render::ui::uirenderer::DrawCommand> {
    let mut commands = Vec::new();
    let mut cursor = pos;
    for message in messages {
        let message_commands = font.text_component(message, font_size);
        for mut cmd in message_commands {
            if let crate::render::ui::uirenderer::DrawCommand::Quad { rect, .. } = &mut cmd {
                rect[0] += cursor;
                rect[1] += cursor;
            } else if let crate::render::ui::uirenderer::DrawCommand::Mesh { vertices, .. } =
                &mut cmd
            {
                for vertex in vertices {
                    vertex.position += cursor.extend(0.0);
                }
            }
            commands.push(cmd);
        }
        let message_size = font.measure_component(message, font_size);
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
