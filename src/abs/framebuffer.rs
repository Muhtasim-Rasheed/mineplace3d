//! Module to work with OpenGL framebuffers.
//!
//! This module provides functionality to create, bind, and manage OpenGL framebuffers.
//! It allows for off-screen rendering and texture attachments.

use std::sync::Arc;

use glow::HasContext;

use crate::abs::Texture;

/// How to use which color channels in the framebuffer.
pub enum ColorUsage {
    /// Use all color channels (RGBA8).
    All,
    /// Use only the red channel floating point (R32F).
    RedFloat,
}

/// Represents an OpenGL framebuffer.
pub struct Framebuffer {
    gl: Arc<glow::Context>,
    fbo: glow::Framebuffer,
    color_tex: Texture,
    depth_tex: Option<Texture>,
}

impl Framebuffer {
    /// Creates a new framebuffer with the specified width and height.
    pub fn new(
        gl: &Arc<glow::Context>,
        width: i32,
        height: i32,
        use_depth: bool,
        color_usage: ColorUsage,
    ) -> Self {
        unsafe {
            let fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

            let color_tex = {
                let tex = gl.create_texture().unwrap();
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));

                let (internal, format, ty) = match color_usage {
                    ColorUsage::All => (glow::RGBA8 as i32, glow::RGBA, glow::UNSIGNED_BYTE),
                    ColorUsage::RedFloat => (glow::R32F as i32, glow::RED, glow::FLOAT),
                };

                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    internal,
                    width,
                    height,
                    0,
                    format,
                    ty,
                    glow::PixelUnpackData::Slice(None),
                );

                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );

                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(tex),
                    0,
                );

                gl.bind_texture(glow::TEXTURE_2D, None);
                tex
            };

            let depth_tex = if use_depth {
                let tex = gl.create_texture().unwrap();
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));

                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::DEPTH_COMPONENT24 as i32, // 24-bit depth
                    width,
                    height,
                    0,
                    glow::DEPTH_COMPONENT,
                    glow::UNSIGNED_INT,
                    glow::PixelUnpackData::Slice(None),
                );

                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::NEAREST as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::NEAREST as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );

                // Attach to framebuffer
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::DEPTH_ATTACHMENT,
                    glow::TEXTURE_2D,
                    Some(tex),
                    0,
                );

                gl.bind_texture(glow::TEXTURE_2D, None);
                Some(tex)
            } else {
                None
            };

            assert!(
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE,
                "Framebuffer incomplete"
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Self {
                gl: gl.clone(),
                fbo,
                color_tex: Texture {
                    gl: gl.clone(),
                    id: color_tex,
                },
                depth_tex: depth_tex.map(|tex| Texture {
                    gl: gl.clone(),
                    id: tex,
                }),
            }
        }
    }

    /// Binds the framebuffer for rendering.
    pub fn bind(&self) {
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        }
    }

    /// Unbinds the framebuffer, reverting to the default framebuffer.
    pub fn unbind(gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    /// Returns the color texture of the framebuffer.
    pub fn texture(&self) -> &Texture {
        &self.color_tex
    }

    /// Returns the depth texture of the framebuffer, if it exists.
    pub fn depth_texture(&self) -> Option<&Texture> {
        self.depth_tex.as_ref()
    }
}
