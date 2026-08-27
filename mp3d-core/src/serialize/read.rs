use glam::{U8Vec3, Vec3};

use crate::{
    registry::{Def, DefId, Registry},
    serialize::Saveable,
};

#[derive(Debug)]
pub enum ReadErrorKind {
    MissingFile(std::path::PathBuf),
    MissingData(String),
    UnexpectedEof { needed: usize, remaining: usize },
    InvalidUtf8,
    InvalidId(String),
    InvalidTag(u8),
    IndexOutOfRange { value: u8, max: u8 },
}

impl std::fmt::Display for ReadErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFile(path) => write!(f, "expected a file at {}", path.display()),
            Self::MissingData(name) => write!(f, "expected a {name}"),
            Self::UnexpectedEof { needed, remaining } => {
                if *needed == 0 {
                    write!(f, "unexpected eof")
                } else {
                    write!(
                        f,
                        "unexpected eof: needed {needed} bytes, {remaining} remaining"
                    )
                }
            }
            Self::InvalidUtf8 => write!(f, "invalid utf-8"),
            Self::InvalidId(id) => write!(f, "invalid registry id: {id}"),
            Self::InvalidTag(t) => write!(f, "invalid tag: {t}"),
            Self::IndexOutOfRange { value, max } => {
                write!(f, "index {value} out of range (maximum {max})")
            }
        }
    }
}

#[derive(Debug)]
pub struct ReadError {
    pub kind: ReadErrorKind,
    pub reason: &'static str,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.reason, self.kind)
    }
}

impl std::error::Error for ReadError {}

pub trait ReadErrorExt<T> {
    fn ctx(self, reason: &'static str) -> Result<T, ReadError>;
}

impl<T> ReadErrorExt<T> for Result<T, ReadErrorKind> {
    fn ctx(self, reason: &'static str) -> Result<T, ReadError> {
        self.map_err(|kind| ReadError { kind, reason })
    }
}

pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], ReadErrorKind> {
        let remaining = self.data.len() - self.pos;
        if remaining < n {
            return Err(ReadErrorKind::UnexpectedEof {
                needed: n,
                remaining,
            });
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, ReadErrorKind> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, ReadErrorKind> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn u32(&mut self) -> Result<u32, ReadErrorKind> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn u64(&mut self) -> Result<u64, ReadErrorKind> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    pub fn i8(&mut self) -> Result<i8, ReadErrorKind> {
        Ok(self.take(1)?[0] as i8)
    }

    pub fn i16(&mut self) -> Result<i16, ReadErrorKind> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn i32(&mut self) -> Result<i32, ReadErrorKind> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn i64(&mut self) -> Result<i64, ReadErrorKind> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    pub fn f32(&mut self) -> Result<f32, ReadErrorKind> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn bool(&mut self) -> Result<bool, ReadErrorKind> {
        Ok(self.u8()? != 0)
    }

    pub fn vec3(&mut self) -> Result<Vec3, ReadErrorKind> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }

    pub fn u8vec3(&mut self) -> Result<U8Vec3, ReadErrorKind> {
        Ok(U8Vec3::new(self.u8()?, self.u8()?, self.u8()?))
    }

    pub fn string(&mut self, len: usize) -> Result<String, ReadErrorKind> {
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ReadErrorKind::InvalidUtf8)
    }

    pub fn load<T: Saveable>(&mut self, version: u8) -> Result<T, ReadError> {
        T::load(self, version)
    }

    pub fn registry_id<I: DefId>(
        &mut self,
        registry: &Registry<impl Def<Id = I>>,
    ) -> Result<I, ReadErrorKind> {
        let ident_len = self.u16()? as usize;
        let ident = self.string(ident_len)?;
        registry
            .get_id(&ident)
            .ok_or(ReadErrorKind::InvalidId(ident))
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}
