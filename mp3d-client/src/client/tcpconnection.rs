use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc,
    thread,
    time::Duration,
};

use mp3d_core::protocol::{C2SMessage, S2CMessage};

use super::Connection;

enum OutboundEvent {
    Message(C2SMessage),
    Flush,
}

pub struct TcpConnection {
    outbound_tx: mpsc::Sender<OutboundEvent>,
    inbound_rx: mpsc::Receiver<S2CMessage>,
    _worker: thread::JoinHandle<()>,
}

impl TcpConnection {
    pub fn connect(addr: impl AsRef<str>) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr.as_ref())?;
        stream.set_read_timeout(Some(Duration::from_millis(10)))?;

        let (outbound_tx, outbound_rx) = mpsc::channel::<OutboundEvent>();
        let (inbound_tx, inbound_rx) = mpsc::channel::<S2CMessage>();

        let worker = thread::Builder::new()
            .name("mp3d-network".into())
            .spawn(move || network_loop(stream, outbound_rx, inbound_tx))
            .expect("failed to spawn network thread");

        Ok(Self {
            outbound_tx,
            inbound_rx,
            _worker: worker,
        })
    }
}

fn network_loop(
    mut stream: TcpStream,
    outbound_rx: mpsc::Receiver<OutboundEvent>,
    inbound_tx: mpsc::Sender<S2CMessage>,
) {
    loop {
        loop {
            match outbound_rx.try_recv() {
                Ok(OutboundEvent::Message(msg)) => {
                    if write_framed(&mut stream, &msg).is_err() {
                        log::warn!("Write failed, closing connection");
                        return;
                    }
                }
                Ok(OutboundEvent::Flush) => {}
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        match read_framed(&mut stream) {
            Ok(Some(bytes)) => match postcard::from_bytes::<S2CMessage>(&bytes) {
                Ok(msg) => {
                    inbound_tx.send(msg).ok();
                }
                Err(e) => log::warn!("Malformed message from server: {e}"),
            },
            Ok(None) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                log::info!("Server closed the connection");
                return;
            }
            Err(e) => {
                log::warn!("Read error: {e}");
                return;
            }
        }
    }
}

fn write_framed(stream: &mut TcpStream, msg: &C2SMessage) -> std::io::Result<()> {
    let bytes = postcard::to_stdvec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    Ok(())
}

fn read_framed(stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok(Some(payload))
}

impl Connection for TcpConnection {
    fn send(&mut self, message: C2SMessage) -> bool {
        if self
            .outbound_tx
            .send(OutboundEvent::Message(message))
            .is_err()
        {
            true
        } else {
            false
        }
    }

    fn flush(&mut self) {
        let _ = self.outbound_tx.send(OutboundEvent::Flush);
    }

    fn receive(&mut self) -> Vec<S2CMessage> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.inbound_rx.try_recv() {
            messages.push(msg);
        }
        messages
    }
}
