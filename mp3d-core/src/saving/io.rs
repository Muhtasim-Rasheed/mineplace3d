//! Small utilities for reading and writing binary data.

#![allow(dead_code)]

use crate::saving::WorldLoadError;

#[inline(always)]
pub fn read_u8<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<u8, WorldLoadError> {
    data_iter.next().ok_or_else(|| {
        WorldLoadError::InvalidSaveFormat(format!("Unexpected end of data while reading {}", ctx))
    })
}

#[inline(always)]
pub fn read_u16<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<u16, WorldLoadError> {
    let low = read_u8(data_iter, ctx)?;
    let high = read_u8(data_iter, ctx)?;
    Ok(u16::from_le_bytes([low, high]))
}

#[inline(always)]
pub fn read_u32<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<u32, WorldLoadError> {
    let b0 = read_u8(data_iter, ctx)?;
    let b1 = read_u8(data_iter, ctx)?;
    let b2 = read_u8(data_iter, ctx)?;
    let b3 = read_u8(data_iter, ctx)?;
    Ok(u32::from_le_bytes([b0, b1, b2, b3]))
}

#[inline(always)]
pub fn read_u64<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<u64, WorldLoadError> {
    let mut bytes = [0; 8];
    for byte in &mut bytes {
        *byte = read_u8(data_iter, ctx)?;
    }
    Ok(u64::from_le_bytes(bytes))
}

#[inline(always)]
pub fn read_i32<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<i32, WorldLoadError> {
    let b0 = read_u8(data_iter, ctx)?;
    let b1 = read_u8(data_iter, ctx)?;
    let b2 = read_u8(data_iter, ctx)?;
    let b3 = read_u8(data_iter, ctx)?;
    Ok(i32::from_le_bytes([b0, b1, b2, b3]))
}

#[inline(always)]
pub fn read_f32<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<f32, WorldLoadError> {
    let b0 = read_u8(data_iter, ctx)?;
    let b1 = read_u8(data_iter, ctx)?;
    let b2 = read_u8(data_iter, ctx)?;
    let b3 = read_u8(data_iter, ctx)?;
    Ok(f32::from_le_bytes([b0, b1, b2, b3]))
}

#[inline(always)]
pub fn read_vec3<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<glam::Vec3, WorldLoadError> {
    let x = read_f32(data_iter, ctx)?;
    let y = read_f32(data_iter, ctx)?;
    let z = read_f32(data_iter, ctx)?;
    Ok(glam::Vec3::new(x, y, z))
}

#[inline(always)]
pub fn read_ivec3<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<glam::IVec3, WorldLoadError> {
    let x = read_i32(data_iter, ctx)?;
    let y = read_i32(data_iter, ctx)?;
    let z = read_i32(data_iter, ctx)?;
    Ok(glam::IVec3::new(x, y, z))
}

#[inline(always)]
pub fn read_u8vec3<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    ctx: &'static str,
) -> Result<glam::U8Vec3, WorldLoadError> {
    let x = read_u8(data_iter, ctx)?;
    let y = read_u8(data_iter, ctx)?;
    let z = read_u8(data_iter, ctx)?;
    Ok(glam::U8Vec3::new(x, y, z))
}

#[inline(always)]
pub fn read_string<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    len: usize,
    ctx: &'static str,
) -> Result<String, WorldLoadError> {
    let bytes = (0..len)
        .map(|_| read_u8(data_iter, ctx))
        .collect::<Result<Vec<u8>, WorldLoadError>>()?;
    String::from_utf8(bytes).map_err(|e| {
        WorldLoadError::InvalidSaveFormat(format!("Invalid UTF-8 string for {}: {}", ctx, e))
    })
}

#[inline(always)]
pub fn take_exact<I: Iterator<Item = u8>>(
    data_iter: &mut I,
    n: usize,
    ctx: &'static str,
) -> Result<Vec<u8>, WorldLoadError> {
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        result.push(read_u8(data_iter, ctx)?);
    }
    Ok(result)
}
