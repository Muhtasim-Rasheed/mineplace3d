use glam::Vec3;

pub struct ByteWriter {
    data: Vec<u8>,
}

impl ByteWriter {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn u8(mut self, v: u8) -> Self {
        self.data.push(v);
        self
    }

    pub fn u16(mut self, v: u16) -> Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u32(mut self, v: u32) -> Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u64(mut self, v: u64) -> Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn f32(mut self, v: f32) -> Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn vec3(self, v: Vec3) -> Self {
        self.f32(v.x).f32(v.y).f32(v.z)
    }

    pub fn bool(self, v: bool) -> Self {
        self.u8(v as u8)
    }

    pub fn string(mut self, s: &str) -> Self {
        self.data.extend_from_slice(&(s.len() as u16).to_le_bytes());
        self.data.extend_from_slice(s.as_bytes());
        self
    }

    pub fn bytes(mut self, s: &[u8]) -> Self {
        self.data.extend_from_slice(s);
        self
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}
