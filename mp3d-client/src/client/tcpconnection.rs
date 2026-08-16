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
    let mut reader = FrameReader::new();

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

        match reader.read_frame(&mut stream) {
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

struct FrameReader {
    len_buf: [u8; 4],
    len_filled: usize,
    payload_buf: Vec<u8>,
    payload_filled: usize,
    payload_len: Option<usize>,
}

impl FrameReader {
    fn new() -> Self {
        Self {
            len_buf: [0u8; 4],
            len_filled: 0,
            payload_buf: Vec::new(),
            payload_filled: 0,
            payload_len: None,
        }
    }

    fn read_frame(&mut self, stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
        if self.payload_len.is_none() {
            while self.len_filled < 4 {
                match stream.read(&mut self.len_buf[self.len_filled..]) {
                    Ok(0) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "connection closed while reading length prefix",
                        ));
                    }
                    Ok(n) => self.len_filled += n,
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
            }
            let len = u32::from_be_bytes(self.len_buf) as usize;
            self.payload_len = Some(len);
            self.payload_buf = vec![0u8; len];
            self.payload_filled = 0;
        }

        let target_len = self.payload_len.unwrap();

        while self.payload_filled < target_len {
            match stream.read(&mut self.payload_buf[self.payload_filled..]) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "connection closed while reading payload",
                    ));
                }
                Ok(n) => self.payload_filled += n,
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
        }

        let payload = std::mem::take(&mut self.payload_buf);
        self.len_filled = 0;
        self.payload_filled = 0;
        self.payload_len = None;
        Ok(Some(payload))
    }
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
