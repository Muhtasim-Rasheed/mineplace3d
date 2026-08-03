use mp3d_core::protocol::{C2SMessage, S2CMessage};

use crate::client::{Connection, localconnection::LocalConnection, tcpconnection::TcpConnection};

pub enum ConnectionKind {
    SinglePlayer {
        connection: LocalConnection,
        world_path: std::path::PathBuf,
    },
    Multiplayer {
        connection: TcpConnection,
    },
}

impl Connection for ConnectionKind {
    fn send(&mut self, message: C2SMessage) -> bool {
        match self {
            ConnectionKind::SinglePlayer { connection, .. } => connection.send(message),
            ConnectionKind::Multiplayer { connection } => connection.send(message),
        }
    }

    fn flush(&mut self) {
        match self {
            ConnectionKind::SinglePlayer { connection, .. } => connection.flush(),
            ConnectionKind::Multiplayer { connection } => connection.flush(),
        }
    }

    fn tick(&mut self, tps: u8) {
        match self {
            ConnectionKind::SinglePlayer { connection, .. } => connection.tick(tps),
            ConnectionKind::Multiplayer { connection } => connection.tick(tps),
        }
    }

    fn receive(&mut self) -> Vec<S2CMessage> {
        match self {
            ConnectionKind::SinglePlayer { connection, .. } => connection.receive(),
            ConnectionKind::Multiplayer { connection } => connection.receive(),
        }
    }
}
