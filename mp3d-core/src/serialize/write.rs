use glam::{U8Vec3, Vec3};

use crate::serialize::Saveable;

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

    pub fn i8(mut self, v: i8) -> Self {
        self.data.push(v as u8);
        self
    }

    pub fn i16(mut self, v: i16) -> Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i32(mut self, v: i32) -> Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i64(mut self, v: i64) -> Self {
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

    pub fn u8vec3(self, v: U8Vec3) -> Self {
        self.u8(v.x).u8(v.y).u8(v.z)
    }

    pub fn bool(self, v: bool) -> Self {
        self.u8(v as u8)
    }

    pub fn string(mut self, s: &str) -> Self {
        self.data.extend_from_slice(s.as_bytes());
        self
    }

    pub fn bytes(mut self, s: &[u8]) -> Self {
        self.data.extend_from_slice(s);
        self
    }

    pub fn save<T: Saveable>(self, val: &T) -> Self {
        val.save(self)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}
