//! The single player scene implementation.

use std::collections::HashMap;

use glam::{IVec3, Vec2, Vec4};
use glow::HasContext;

use crate::{
    abs::{Mesh, ShaderProgram},
    client::{Client, Connection, LocalConnection},
    render::meshing::mesh_world,
    shader_program,
};

/// The [`SinglePlayer`] struct represents the single player scene.
pub struct SinglePlayer {
    client: Client<LocalConnection>,
    chunk_meshes: HashMap<IVec3, Mesh>,
    chunk_shader: ShaderProgram,
    // aspect_ratio: f32,
    width: u32,
    height: u32,
    tick_acc: f32,
    /// At most 48 ticks per second, but can be lower if the client can't keep up.
    tick_rate: f32,
    playing: bool,
}

impl SinglePlayer {
    /// Creates a new [`SinglePlayer`] instance.
    pub fn new(gl: &std::sync::Arc<glow::Context>) -> Self {
        let server = mp3d_core::server::Server::new();
        let connection = LocalConnection::new(server);
        let client = Client::new(connection);
        let chunk_shader = shader_program!(chunk, gl, "..");
        Self {
            client,
            chunk_meshes: HashMap::new(),
            chunk_shader,
            width: 1280,
            height: 720,
            tick_acc: 0.0,
            tick_rate: 48.0,
            playing: true,
        }
    }
}

impl super::Scene for SinglePlayer {
    fn handle_event(&mut self, gl: &std::sync::Arc<glow::Context>, event: &sdl2::event::Event) {
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
        {
            self.playing = !self.playing;
        }
    }

    fn update(
        &mut self,
        gl: &std::sync::Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        window: &mut sdl2::video::Window,
        sdl_ctx: &sdl2::Sdl,
    ) -> super::SceneSwitch {
        window.set_title(&format!("Mineplace3D - Single Player - FPS: {:.2}", 1.0 / ctx.delta_time))
            .unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(self.playing);
        if self.playing {
            self.client.send_input(ctx, self.tick_rate as u8);
            // On single player we do not recieve messages from the server.
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
        }
        mesh_world(gl, &mut self.client.world, &mut self.chunk_meshes);
        super::SceneSwitch::None
    }

    fn render(
        &mut self,
        gl: &std::sync::Arc<glow::Context>,
        ui: &mut crate::render::ui::uirenderer::UIRenderer,
    ) {
        unsafe {
            gl.enable(glow::DEPTH_TEST);
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
                self.client.player.projection(self.width as f32 / self.height as f32),
            );
            for mesh in self.chunk_meshes.values() {
                mesh.draw();
            }

            if !self.playing {
                ui.add_command(crate::render::ui::uirenderer::DrawCommand {
                    rect: [Vec2::new(0.0, 0.0), Vec2::new(self.width as f32, self.height as f32)],
                    uv_rect: [Vec2::ZERO, Vec2::ONE],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::new(0.0, 0.0, 0.0, 0.5)),
                });
                ui.finish();
            }
        }
    }
}
