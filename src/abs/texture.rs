//! Structs and functions for handling textures.
//!
//! The module provides the [`Texture`] struct which is a CPU representation of a GPU texture.

use std::sync::Arc;

use glow::HasContext;
use image::{DynamicImage, GenericImageView};

/// Represents a texture stored on the GPU side.
pub struct Texture {
    pub(super) gl: Arc<glow::Context>,
    pub(super) id: glow::Texture,
}

impl Texture {
    /// Creates a new texture from the given [`image::DynamicImage`].
    pub fn new(gl: &Arc<glow::Context>, image: &DynamicImage) -> Self {
        let (width, height) = image.dimensions();
        let data = image.to_rgba8().into_raw();
        unsafe {
            let texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data.as_slice())),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST_MIPMAP_NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            Self {
                gl: Arc::clone(gl),
                id: texture,
            }
        }
    }

    // Creates a new texture from the given raw RGBA data.
    pub fn new_from_data(gl: &Arc<glow::Context>, width: u32, height: u32, data: &[u8]) -> Self {
        unsafe {
            let texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST_MIPMAP_NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            Self {
                gl: Arc::clone(gl),
                id: texture,
            }
        }
    }

    /// Binds the texture to the specified texture unit.
    pub fn bind_to_unit(&self, unit: u32) {
        unsafe {
            self.gl.active_texture(glow::TEXTURE0 + unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_texture(self.id);
        }
    }
}
