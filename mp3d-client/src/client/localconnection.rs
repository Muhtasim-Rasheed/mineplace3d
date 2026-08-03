use mp3d_core::{
    protocol::{C2SMessage, S2CMessage},
    server::Server,
};

use crate::client::Connection;

/// A local connection that directly interacts with a server instance.
///
/// The [`LocalConnection`] owns the server instance instead of borrowing it. The local connection
/// will use a connection ID of `0` for all interactions since it is the only connection, and the
/// server does not need to differentiate between multiple clients.
pub struct LocalConnection {
    pub server: Server,
    pub message: Option<S2CMessage>,
}

impl LocalConnection {
    /// Creates a new `LocalConnection` with the given server and user ID.
    pub fn new(server: Server) -> Self {
        log::info!("Creating local connection");

        Self {
            server,
            message: None,
        }
    }
}

impl Connection for LocalConnection {
    fn send(&mut self, message: C2SMessage) -> bool {
        if let Some(message) = self.server.handle_message(0, message) {
            self.message = Some(message);
        }
        false
    }

    // All messages are sent immediately to the server, so nothing is to be done
    fn flush(&mut self) {}

    fn tick(&mut self, tps: u8) {
        self.server.tick(tps);
    }

    fn receive(&mut self) -> Vec<S2CMessage> {
        if let Some(message) = self.message.take() {
            vec![message]
        } else if let Some(user_id) = self.server.connections.get(&0)
            && let Some(session) = self.server.sessions.get_mut(user_id)
        {
            std::mem::take(&mut session.pending_messages)
        } else {
            vec![]
        }
    }
}
