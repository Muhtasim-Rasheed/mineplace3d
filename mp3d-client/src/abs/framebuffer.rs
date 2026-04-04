//! Module to work with OpenGL framebuffers.
//!
//! This module provides functionality to create, bind, and manage OpenGL framebuffers.
//! It allows for off-screen rendering and texture attachments.

use std::sync::Arc;

use glow::HasContext;

use crate::abs::Texture;

/// How to use which color channels in the framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ColorUsage {
    /// Use all color channels as 8-bit unsigned integers.
    RGBA8,
    /// Use only the red channel as a byte.
    R8,
    /// Use only the red, green, and blue channels as 16-bit floats.
    RGB16F,
    /// Use only the red channel as a 32-bit float.
    R32F,
}

/// Represents an OpenGL framebuffer.
pub struct Framebuffer {
    gl: Arc<glow::Context>,
    fbo: glow::Framebuffer,
    color_texes: Vec<Texture>,
    depth_tex: Option<Texture>,
    color_usages: Vec<ColorUsage>,
    width: i32,
    height: i32,
}

impl Framebuffer {
    /// Creates a new framebuffer with the specified width and height.
    pub fn new(
        gl: &Arc<glow::Context>,
        width: i32,
        height: i32,
        use_depth: bool,
        color_usages: &[ColorUsage],
    ) -> Self {
        unsafe {
            let fbo = gl.create_framebuffer().unwrap();
            log::info!(
                "Creating framebuffer: size={}x{}, use_depth={}, color_usages={:?}",
                width,
                height,
                use_depth,
                color_usages
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

            let color_texes = color_usages
                .iter()
                .enumerate()
                .map(|(i, color_usage)| {
                    let tex = gl.create_texture().unwrap();
                    gl.bind_texture(glow::TEXTURE_2D, Some(tex));

                    let (internal, format, ty) = match color_usage {
                        ColorUsage::RGBA8 => (glow::RGBA8 as i32, glow::RGBA, glow::UNSIGNED_BYTE),
                        ColorUsage::R8 => (glow::R8 as i32, glow::RED, glow::UNSIGNED_BYTE),
                        ColorUsage::RGB16F => (glow::RGB16F as i32, glow::RGB, glow::HALF_FLOAT),
                        ColorUsage::R32F => (glow::R32F as i32, glow::RED, glow::FLOAT),
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

                    // Attach to framebuffer
                    let attachment = glow::COLOR_ATTACHMENT0 + i as u32;
                    gl.framebuffer_texture_2d(
                        glow::FRAMEBUFFER,
                        attachment,
                        glow::TEXTURE_2D,
                        Some(tex),
                        0,
                    );

                    gl.bind_texture(glow::TEXTURE_2D, None);
                    Texture {
                        gl: gl.clone(),
                        id: tex,
                        width: width as u32,
                        height: height as u32,
                    }
                })
                .collect::<Vec<_>>();

            gl.draw_buffers(
                &color_usages
                    .iter()
                    .enumerate()
                    .map(|(i, _)| glow::COLOR_ATTACHMENT0 + i as u32)
                    .collect::<Vec<_>>(),
            );

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
                Some(Texture {
                    gl: gl.clone(),
                    id: tex,
                    width: width as u32,
                    height: height as u32,
                })
            } else {
                None
            };

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                panic!("Framebuffer incomplete: status={:#X}", status);
            }

            if use_depth {
                log::info!(
                    "Framebuffer with {} color attachment(s) and depth attachment created successfully",
                    color_texes.len()
                );
            } else {
                log::info!(
                    "Framebuffer with {} color attachment(s) created successfully",
                    color_texes.len()
                );
            }

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Self {
                gl: gl.clone(),
                fbo,
                color_texes,
                depth_tex,
                color_usages: color_usages.to_vec(),
                width,
                height,
            }
        }
    }

    /// Binds the framebuffer for rendering.
    pub fn bind(&self) {
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            self.gl.viewport(0, 0, self.width, self.height);
        }
    }

    /// Unbinds the framebuffer, reverting to the default framebuffer.
    pub fn unbind(gl: &glow::Context, width: i32, height: i32) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, width, height);
        }
    }

    /// Resizes the framebuffer to the specified width and height.
    pub fn resize(&mut self, width: i32, height: i32) {
        unsafe {
            for (i, color_tex) in self.color_texes.iter().enumerate() {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(color_tex.id));
                let (internal, format, ty) = match self.color_usages[i] {
                    ColorUsage::RGBA8 => (glow::RGBA8 as i32, glow::RGBA, glow::UNSIGNED_BYTE),
                    ColorUsage::R8 => (glow::R8 as i32, glow::RED, glow::UNSIGNED_BYTE),
                    ColorUsage::RGB16F => (glow::RGB16F as i32, glow::RGB, glow::HALF_FLOAT),
                    ColorUsage::R32F => (glow::R32F as i32, glow::RED, glow::FLOAT),
                };
                self.gl.tex_image_2d(
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
                self.gl.bind_texture(glow::TEXTURE_2D, None);
                if let Some(depth_tex) = &self.depth_tex {
                    self.gl.bind_texture(glow::TEXTURE_2D, Some(depth_tex.id));
                    self.gl.tex_image_2d(
                        glow::TEXTURE_2D,
                        0,
                        glow::DEPTH_COMPONENT24 as i32,
                        width,
                        height,
                        0,
                        glow::DEPTH_COMPONENT,
                        glow::UNSIGNED_INT,
                        glow::PixelUnpackData::Slice(None),
                    );
                    self.gl.bind_texture(glow::TEXTURE_2D, None);
                }
                self.gl.viewport(0, 0, width, height);
                self.width = width;
                self.height = height;
            }
        }
    }

    /// Returns the color texture of the framebuffer.
    pub fn textures(&self) -> &[Texture] {
        &self.color_texes
    }

    /// Returns the depth texture of the framebuffer, if it exists.
    #[allow(unused)]
    pub fn depth_texture(&self) -> Option<&Texture> {
        self.depth_tex.as_ref()
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_framebuffer(self.fbo);
            // The textures are dropped in their respective Drop implementations
        }
    }
}
