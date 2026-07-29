use glam::Vec3;

use crate::registry::{Def, DefId, Registry};

#[derive(Debug)]
pub enum ReadError {
    UnexpectedEof { needed: usize, remaining: usize },
    InvalidUtf8,
    InvalidId(String),
    InvalidTag(u8),
    IndexOutOfRange { value: u8, max: u8 },
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEof { needed, remaining } => write!(
                f,
                "unexpected eof: needed {needed} bytes, {remaining} remaining"
            ),
            Self::InvalidUtf8 => write!(f, "invalid utf-8"),
            Self::InvalidId(id) => write!(f, "invalid registry id: {id}"),
            Self::InvalidTag(t) => write!(f, "invalid enum tag: {t}"),
            Self::IndexOutOfRange { value, max } => {
                write!(f, "Index {value} out of range (maximum {max})")
            }
        }
    }
}
impl std::error::Error for ReadError {}

pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], ReadError> {
        let remaining = self.data.len() - self.pos;
        if remaining < n {
            return Err(ReadError::UnexpectedEof {
                needed: n,
                remaining,
            });
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, ReadError> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, ReadError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn u32(&mut self) -> Result<u32, ReadError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn u64(&mut self) -> Result<u64, ReadError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    pub fn f32(&mut self) -> Result<f32, ReadError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn bool(&mut self) -> Result<bool, ReadError> {
        Ok(self.u8()? != 0)
    }

    pub fn vec3(&mut self) -> Result<Vec3, ReadError> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }

    pub fn string(&mut self) -> Result<String, ReadError> {
        let len = self.u16()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ReadError::InvalidUtf8)
    }

    pub fn registry_id<I: DefId>(
        &mut self,
        registry: &Registry<impl Def<Id = I>>,
    ) -> Result<I, ReadError> {
        let ident = self.string()?;
        registry.get_id(&ident).ok_or(ReadError::InvalidId(ident))
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}
