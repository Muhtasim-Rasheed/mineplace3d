//! The single player scene implementation.

use std::{collections::HashMap, rc::Rc, sync::Arc};

use glam::{IVec3, Vec2, Vec4};
use glow::HasContext;
use mp3d_core::TextComponent;

use crate::{
    abs::{Mesh, ShaderProgram, TextureHandle},
    client::{Client, Connection, LocalConnection},
    render::{
        meshing::mesh_world,
        ui::widgets::{Button, Column, Font, Label, Widget},
    },
    shader_program,
};

/// The [`SinglePlayer`] struct represents the single player scene.
pub struct SinglePlayer {
    client: Client<LocalConnection>,
    chunk_meshes: HashMap<IVec3, Mesh>,
    chunk_shader: ShaderProgram,
    width: u32,
    height: u32,
    tick_acc: f32,
    tick_rate: f32,
    playing: bool,
    chat_input_label: Option<Label>,
    pause_screen: Column,
    font: Rc<Font>,
}

impl SinglePlayer {
    /// Creates a new [`SinglePlayer`] instance.
    pub fn new(
        gl: &Arc<glow::Context>,
        font: &Rc<Font>,
        gui_tex: TextureHandle,
        window_size: (u32, u32),
    ) -> Self {
        let server = mp3d_core::server::Server::new();
        let connection = LocalConnection::new(server);
        let client = Client::new(connection);
        let chunk_shader = shader_program!(chunk, gl, "..");

        let return_to_game = Button::new(
            "Return to Game",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );
        let main_menu = Button::new(
            "Main Menu",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );
        let mut pause_screen = Column::new(
            20.0,
            crate::render::ui::widgets::Alignment::Center,
            Vec4::ZERO,
            crate::render::ui::widgets::Justification::Center,
        );
        pause_screen.add_widget(return_to_game);
        pause_screen.add_widget(main_menu);
        Self {
            client,
            chunk_meshes: HashMap::new(),
            chunk_shader,
            width: window_size.0,
            height: window_size.1,
            tick_acc: 0.0,
            tick_rate: 48.0,
            playing: true,
            chat_input_label: None,
            pause_screen,
            font: font.clone(),
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
            self.width = *width as u32;
            self.height = *height as u32;
            unsafe {
                gl.viewport(0, 0, *width, *height);
            }
        }
        if let sdl2::event::Event::KeyDown { keycode, .. } = event
            && *keycode == Some(sdl2::keyboard::Keycode::Escape)
            && !self.client.chat_open
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
    ) -> super::SceneSwitch {
        window.set_title("Mineplace3D - Single Player").unwrap();
        sdl_ctx
            .mouse()
            .set_relative_mouse_mode(self.playing && !self.client.chat_open);
        // On single player while the game is paused we do not recieve messages from the server.
        if self.playing {
            self.client.send_input(ctx, self.tick_rate as u8);
            self.client.recieve_state();
            let tick_time = 1.0 / self.tick_rate;
            self.tick_acc += ctx.delta_time;
            if self.tick_acc > tick_time * 5.0 {
                // If the client is really lagging, we don't want to try to catch up on all the ticks, as that would cause even more lag...
                self.tick_acc = tick_time * 5.0;
            }
            while self.tick_acc >= tick_time {
                self.client.connection.tick(self.tick_rate as u8);
                self.tick_acc -= tick_time;
            }
        } else {
            self.pause_screen.update(ctx);
            self.pause_screen
                .layout(&crate::render::ui::widgets::LayoutContext {
                    max_size: Vec2::new(self.width as f32, self.height as f32),
                    cursor: Vec2::ZERO,
                });
            if self
                .pause_screen
                .get_widget::<Button>(0)
                .is_some_and(|btn| btn.is_released())
            {
                self.playing = true;
            }
            if self
                .pause_screen
                .get_widget::<Button>(1)
                .is_some_and(|btn| btn.is_released())
            {
                return super::SceneSwitch::Pop;
            }
        }
        if let Some(chat) = self.client.chat_message.as_ref() {
            if self.chat_input_label.is_none() {
                self.chat_input_label = Some(Label::new(chat, 24.0, Vec4::ONE, &self.font));
            } else {
                self.chat_input_label.as_mut().unwrap().text = chat.clone();
            }
        } else {
            self.chat_input_label = None;
        }
        if let Some(label) = self.chat_input_label.as_mut() {
            label.update(ctx);
            label.layout(&crate::render::ui::widgets::LayoutContext {
                max_size: Vec2::new(self.width as f32, self.height as f32),
                cursor: Vec2::new(10.0, self.height as f32 - 34.0),
            });
        }
        let unloaded = self
            .client
            .world
            .unload_chunks(self.client.player.position.as_ivec3());
        for pos in unloaded {
            self.chunk_meshes.remove(&pos);
        }
        mesh_world(gl, &mut self.client.world, &mut self.chunk_meshes);
        super::SceneSwitch::None
    }

    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut crate::render::ui::uirenderer::UIRenderer,
    ) {
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.clear_color(0.1, 0.1, 0.2, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.chunk_shader.use_program();
            self.chunk_shader
                .set_uniform("u_view", self.client.player.view());
            self.chunk_shader.set_uniform(
                "u_projection",
                self.client
                    .player
                    .projection(self.width as f32 / self.height as f32),
            );
            for mesh in self.chunk_meshes.values() {
                mesh.draw();
            }

            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.depth_mask(false);

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
            let message_size = measure_messages(&self.font, &messages, 24.0);

            let mut messages_start_y = self.height as f32 - message_size.y - 10.0;

            if let Some(chat) = self.chat_input_label.as_ref() {
                messages_start_y -= 34.0 + 15.0;
                let label_size = chat.size_hint();
                ui.add_command(crate::render::ui::uirenderer::DrawCommand {
                    rect: [
                        Vec2::new(5.0, self.height as f32 - label_size.y - 15.0),
                        Vec2::new(5.0 + label_size.x + 10.0, self.height as f32 - 5.0),
                    ],
                    uv_rect: [Vec2::ZERO, Vec2::ONE],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(
                        0.0, 0.0, 0.0, 0.5,
                    )),
                });
                ui.finish();
                chat.draw(ui);
            }

            ui.add_command(crate::render::ui::uirenderer::DrawCommand {
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
            });
            for cmd in text_messages(
                &self.font,
                &messages,
                24.0,
                Vec2::new(10.0, messages_start_y),
            ) {
                ui.add_command(cmd);
            }
            ui.finish();

            if !self.playing {
                ui.add_command(crate::render::ui::uirenderer::DrawCommand {
                    rect: [
                        Vec2::new(0.0, 0.0),
                        Vec2::new(self.width as f32, self.height as f32),
                    ],
                    uv_rect: [Vec2::ZERO, Vec2::ONE],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(
                        0.0, 0.0, 0.0, 0.5,
                    )),
                });
                ui.finish();

                self.pause_screen.draw(ui);
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
            cmd.rect[0] += cursor;
            cmd.rect[1] += cursor;
            commands.push(cmd);
        }
        let message_size = font.measure_component(message, font_size);
        cursor.y += message_size.y;
    }
    commands
}
