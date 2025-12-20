//! UI rendering utilities, including quad mesh and bitmap font support.
//!
//! This module provides structures and functions for rendering 2D UI elements
//! in a 3D graphics application. It includes a vertex structure for UI rendering,
//! a function to create a quad mesh, and a bitmap font structure for rendering text.

use std::sync::Arc;

use crate::{abs::Vertex, mesh::Mesh};
use glam::*;
use glow::HasContext;
use image::DynamicImage;

/// Vertex structure for UI rendering.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UIVertex {
    pub position: Vec3,
    pub uv: Vec2,
}

impl Vertex for UIVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<UIVertex>() as i32;

            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(0);

            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                stride,
                std::mem::size_of::<Vec3>() as i32,
            );
            gl.enable_vertex_attrib_array(1);
        }
    }
}

/// Creates a quad mesh for UI rendering.
pub fn quad_mesh(gl: &Arc<glow::Context>) -> Mesh {
    let vertices: [UIVertex; 4] = [
        UIVertex {
            position: vec3(-1.0, -1.0, 0.0),
            uv: vec2(0.0, 0.0),
        },
        UIVertex {
            position: vec3(1.0, -1.0, 0.0),
            uv: vec2(1.0, 0.0),
        },
        UIVertex {
            position: vec3(1.0, 1.0, 0.0),
            uv: vec2(1.0, 1.0),
        },
        UIVertex {
            position: vec3(-1.0, 1.0, 0.0),
            uv: vec2(0.0, 1.0),
        },
    ];
    let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
    Mesh::new(gl, &vertices, &indices, glow::TRIANGLES)
}

/// Bitmap font structure for rendering text.
pub struct BitmapFont {
    first_char: char,
    chars_per_row: u32,
    char_width: u32,
    char_height: u32,
    pub atlas: DynamicImage,
}

impl BitmapFont {
    /// Creates a new bitmap font from the given parameters.
    pub fn new(
        atlas: DynamicImage,
        first_char: char,
        chars_per_row: u32,
        char_width: u32,
        char_height: u32,
    ) -> Self {
        BitmapFont {
            first_char,
            chars_per_row,
            char_width,
            char_height,
            atlas,
        }
    }

    /// Gets the UV coordinates for the given character.
    pub fn get_glyph_uv(&self, ch: char) -> Option<([f32; 2], [f32; 2])> {
        let glyph_index = ch as u32 - self.first_char as u32;

        let tex_width = self.atlas.width();
        let tex_height = self.atlas.height();

        if glyph_index >= self.chars_per_row * (tex_height / self.char_height) {
            return None; // glyph not in atlas
        }

        let col = glyph_index % self.chars_per_row;
        let row = glyph_index / self.chars_per_row;

        let u0 = (col * self.char_width) as f32 / tex_width as f32;
        let v0 = (row * self.char_height) as f32 / tex_height as f32;
        let u1 = ((col + 1) * self.char_width) as f32 / tex_width as f32;
        let v1 = ((row + 1) * self.char_height) as f32 / tex_height as f32;

        Some(([u0, v0], [u1, v1]))
    }

    /// Calculates the width and height of the given text string at the specified font size.
    pub fn text_metrics(&self, text: &str, font_size: f32) -> (f32, f32) {
        let mut max_width = 0f32;
        let mut current_width = 0.0;
        let mut lines = 1;

        for ch in text.chars() {
            if ch == '\n' {
                max_width = max_width.max(current_width);
                current_width = 0.0;
                lines += 1;
                continue;
            }

            if self.get_glyph_uv(ch).is_some() {
                let w = self.char_width as f32 * font_size / self.char_height as f32;
                current_width += w;
            }
        }

        max_width = max_width.max(current_width);
        let total_height = lines as f32 * font_size;

        (max_width, total_height)
    }

    /// Builds a mesh for rendering the given text string at the specified position and font size.
    pub fn build(
        &self,
        gl: &Arc<glow::Context>,
        text: &str,
        start_x: f32,
        start_y: f32,
        font_size: f32,
    ) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let mut x = start_x;
        let mut y = start_y;
        let mut i = 0;

        for ch in text.chars() {
            if ch == '\n' {
                x = start_x;
                y += font_size;
                continue;
            }

            if let Some((uv0, uv1)) = self.get_glyph_uv(ch) {
                let h = font_size;
                let w = self.char_width as f32 * font_size / self.char_height as f32;

                let idx = i * 4;

                vertices.push(UIVertex {
                    position: glam::vec3(x, y + h, 0.0),
                    uv: glam::vec2(uv0[0], uv1[1]),
                }); // Top-left
                vertices.push(UIVertex {
                    position: glam::vec3(x + w, y + h, 0.0),
                    uv: glam::vec2(uv1[0], uv1[1]),
                }); // Top-right
                vertices.push(UIVertex {
                    position: glam::vec3(x + w, y, 0.0),
                    uv: glam::vec2(uv1[0], uv0[1]),
                }); // Bottom-right
                vertices.push(UIVertex {
                    position: glam::vec3(x, y, 0.0),
                    uv: glam::vec2(uv0[0], uv0[1]),
                }); // Bottom-left

                indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

                x += w; // Advance cursor
                i += 1;
            }
        }

        Mesh::new(gl, &vertices, &indices, glow::TRIANGLES)
    }
}
