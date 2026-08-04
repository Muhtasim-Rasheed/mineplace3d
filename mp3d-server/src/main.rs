use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use mp3d_core::{
    protocol::{C2SMessage, S2CMessage},
    server::Server,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::MissedTickBehavior,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::config::ServerConfig;

mod config;

const TICK_RATE: f64 = 48.0;
const TICK_DURATION: Duration = Duration::from_nanos((1_000_000_000.0 / TICK_RATE) as u64);

/// Server software for Mineplace3D.
#[derive(argh::FromArgs, Debug)]
struct Args {
    /// configuration file to use
    #[argh(option, short = 'C')]
    pub config: PathBuf,

    /// where to output log files
    #[argh(option, short = 'l', default = "PathBuf::from(\"logs\")")]
    pub logs: PathBuf,

    /// where to save or load the world
    #[argh(option, short = 's', default = "PathBuf::from(\"\")")]
    pub save_path: PathBuf,
}

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(0);
static CURRENT_CLIENTS: AtomicU64 = AtomicU64::new(0);
static CONFIG: OnceLock<ServerConfig> = OnceLock::new();

fn config() -> &'static ServerConfig {
    CONFIG
        .get()
        .expect("config() used before config initialization")
}

#[derive(Debug)]
enum ServerEvent {
    Connected {
        connection_id: u64,
        outbound: mpsc::UnboundedSender<S2CMessage>,
    },
    Disconnected {
        connection_id: u64,
    },
    Message {
        connection_id: u64,
        message: C2SMessage,
    },
}

struct LogStop;
impl Drop for LogStop {
    fn drop(&mut self) {
        log::info!("Stopping!");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = argh::from_env();

    if !args.logs.exists() {
        std::fs::create_dir_all(&args.logs)?;
    }

    let log_file_path = args.logs.join("game.log");

    if log_file_path.exists() {
        let birth = std::fs::metadata(&log_file_path)
            .and_then(|meta| meta.created())
            .unwrap_or_else(|_| std::time::SystemTime::now());
        let timestamp = chrono::DateTime::<chrono::Local>::from(birth).format("%Y-%m-%d_%H-%M-%S");
        let new_log_file_path = args.logs.join(format!("game-{}.log", timestamp));
        std::fs::rename(&log_file_path, new_log_file_path).expect("Failed to rotate log file");
    }

    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_file_path)?)
        .apply()?;

    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "non-string panic payload"
        };

        log::error!("panic at {}: {}", location, payload);
        log::error!("backtrace:\n{}", std::backtrace::Backtrace::force_capture());
    }));

    mp3d_core::init();

    log::info!("Mineplace3D {}", env!("CARGO_PKG_VERSION"));
    log::info!("Reading config...");
    CONFIG
        .set(match config::read_config(&args.config) {
            Ok(conf) => conf,
            Err(e) => {
                log::error!("Invalid configuration: {e}.");
                error_exit()
            }
        })
        .unwrap();

    log::info!("Loading server...");
    let mut server = if args.save_path.exists() {
        match Server::load(false, args.save_path.clone()) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Broken world save: {e}.");
                error_exit()
            }
        }
    } else {
        Server::new(false, 0, args.save_path.clone())
    };

    let listener = TcpListener::bind(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        config().port.unwrap_or(8080),
    ))
    .await?;
    log::info!("Listening on 127.0.0.1:{}.", config().port.unwrap_or(8080));

    let _log_stop = LogStop;

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ServerEvent>();

    {
        let event_tx = event_tx.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let current = CURRENT_CLIENTS.fetch_add(1, Ordering::SeqCst);
                        if current >= config().max_clients as u64 {
                            CURRENT_CLIENTS.fetch_sub(1, Ordering::SeqCst);
                            log::info!(
                                "Rejecting connection from {addr}: server full ({current}/{})",
                                config().max_clients
                            );
                            drop(stream);
                            continue;
                        }

                        let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
                        log::info!("Connection {connection_id} from {addr}");
                        let tx = event_tx.clone();
                        tokio::spawn(handle_connection(connection_id, stream, tx));
                    }
                    Err(e) => log::error!("Accept failed: {e}"),
                }
            }
        });
    }

    let mut outbound_senders: HashMap<u64, mpsc::UnboundedSender<S2CMessage>> = HashMap::new();
    let mut session_to_connection: HashMap<u64, u64> = HashMap::new();

    let mut interval = tokio::time::interval(TICK_DURATION);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                while let Ok(event) = event_rx.try_recv() {
                    match event {
                        ServerEvent::Connected {
                            connection_id,
                            outbound,
                        } => {
                            outbound_senders.insert(connection_id, outbound);
                        }
                        ServerEvent::Disconnected { connection_id } => {
                            outbound_senders.remove(&connection_id);
                            if let Some(session_id) = server.connections.remove(&connection_id) {
                                session_to_connection.remove(&session_id);
                            }
                        }
                        ServerEvent::Message {
                            connection_id,
                            message,
                        } => {
                            if let Some(reply) = server.handle_message(connection_id, message) {
                                if let Some(sender) = outbound_senders.get(&connection_id) {
                                    sender.send(reply).ok();
                                }
                            }
                            if let Some(&session_id) = server.connections.get(&connection_id) {
                                session_to_connection
                                    .entry(session_id)
                                    .or_insert(connection_id);
                            }
                        }
                    }
                }

                server.tick(TICK_RATE as u8);

                for (session_id, session) in server.sessions.iter_mut() {
                    if session.pending_messages.is_empty() {
                        continue;
                    }
                    if let Some(&connection_id) = session_to_connection.get(session_id) {
                        if let Some(sender) = outbound_senders.get(&connection_id) {
                            for msg in session.pending_messages.drain(..) {
                                sender.send(msg).ok();
                            }
                        }
                    } else {
                        session.pending_messages.clear();
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                log::info!("Ctrl+C received, shutting down...");
                break;
            }
        }
    }

    log::info!("Saving world...");
    if !args.save_path.exists() {
        std::fs::create_dir_all(&args.save_path)?;
    }
    if let Err(e) = server.save() {
        log::error!("Failed to save world on shutdown: {e}");
    }

    Ok(())
}

fn error_exit() -> ! {
    log::error!("Cannot continue. Sorry.");
    std::process::exit(1);
}

async fn handle_connection(
    connection_id: u64,
    stream: TcpStream,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
) {
    let (reader, writer) = stream.into_split();
    let mut framed_reader = Framed::new(reader, LengthDelimitedCodec::new());
    let mut framed_writer = Framed::new(writer, LengthDelimitedCodec::new());

    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<S2CMessage>();
    event_tx
        .send(ServerEvent::Connected {
            connection_id,
            outbound: outbound_tx,
        })
        .ok();

    let writer_task = tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            let bytes = match postcard::to_stdvec(&msg) {
                Ok(b) => b,
                Err(e) => {
                    log::error!("Failed to serialize S2CMessage: {e}");
                    continue;
                }
            };
            if framed_writer.send(bytes.into()).await.is_err() {
                break;
            }
        }
    });

    while let Some(frame) = framed_reader.next().await {
        match frame {
            Ok(bytes) => match postcard::from_bytes::<C2SMessage>(&bytes) {
                Ok(message) => {
                    event_tx
                        .send(ServerEvent::Message {
                            connection_id,
                            message,
                        })
                        .ok();
                }
                Err(e) => {
                    log::warn!("Connection {connection_id} sent malformed message: {e}");
                    break;
                }
            },
            Err(e) => {
                log::warn!("Connection {connection_id} read error: {e}");
                break;
            }
        }
    }

    event_tx
        .send(ServerEvent::Disconnected { connection_id })
        .ok();
    writer_task.abort();
    CURRENT_CLIENTS.fetch_sub(1, Ordering::SeqCst);
}
