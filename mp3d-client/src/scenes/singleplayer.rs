//! The single player scene implementation.

use crate::client::{Client, LocalConnection};

/// The [`SinglePlayer`] struct represents the single player scene.
pub struct SinglePlayer {
    client: Client<LocalConnection>,
}

impl SinglePlayer {
    /// Creates a new [`SinglePlayer`] instance.
    pub fn new() -> Self {
        let server = mp3d_core::server::Server::new();
        let connection = LocalConnection::new(server);
        let client = Client::new(connection);
        Self {
            client,
        }
    }
}

impl super::Scene for SinglePlayer {
    fn update(
        &mut self,
        ctx: &crate::other::UpdateContext,
        _window: &sdl2::video::Window,
    ) -> super::SceneSwitch {
        self.client.send_input(ctx);
        super::SceneSwitch::None
    }

    fn render(&mut self, gl: &std::sync::Arc<glow::Context>, ui: &mut crate::render::ui::uirenderer::UIRenderer) {
        // uhhhhhh
    }
}
