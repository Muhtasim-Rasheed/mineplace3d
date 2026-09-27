use glam::IVec3;
use std::collections::HashSet;
use std::sync::{Arc, mpsc};

use crate::world::chunk::Chunk;
use crate::world::generation::Generator;

pub struct GenerationPool {
    pub generator: Arc<Generator>,
    request_tx: mpsc::Sender<(u64, IVec3)>,
    result_rx: mpsc::Receiver<(u64, IVec3, Chunk)>,
    pending: HashSet<IVec3>,
}

impl GenerationPool {
    pub fn new(generator: Generator, num_workers: usize) -> Self {
        let generator = Arc::new(generator);
        let (request_tx, request_rx) = mpsc::channel::<(u64, IVec3)>();
        let (result_tx, result_rx) = mpsc::channel();
        let request_rx = Arc::new(std::sync::Mutex::new(request_rx));

        for _ in 0..num_workers {
            let request_rx = Arc::clone(&request_rx);
            let result_tx = result_tx.clone();
            let generator = Arc::clone(&generator);

            std::thread::spawn(move || {
                loop {
                    let job = { request_rx.lock().unwrap().recv() };
                    match job {
                        Ok((session_id, pos)) => {
                            let chunk = generator.generate_chunk(pos);
                            let _ = result_tx.send((session_id, pos, chunk));
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        Self {
            generator,
            request_tx,
            result_rx,
            pending: HashSet::new(),
        }
    }

    /// Called from the message handler when a chunk is requested.
    pub fn request_chunk(&mut self, session_id: u64, pos: IVec3) {
        if self.pending.insert(pos) {
            let _ = self.request_tx.send((session_id, pos));
        }
    }

    /// Called once per tick. Drains whatever finished, non-blocking.
    /// `max` caps how many chunks you'll send out in one tick.
    pub fn drain_ready(&mut self, max: usize) -> Vec<(u64, IVec3, Chunk)> {
        let mut out = Vec::new();
        while out.len() < max {
            match self.result_rx.try_recv() {
                Ok((session_id, pos, chunk)) => {
                    self.pending.remove(&pos);
                    out.push((session_id, pos, chunk));
                }
                Err(_) => break,
            }
        }
        out
    }
}
