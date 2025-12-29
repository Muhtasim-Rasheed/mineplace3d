//! UI rendering utilities, including quad mesh and bitmap font support.
//!
//! This module provides structures and functions for rendering 2D UI elements
//! in a 3D graphics application. It includes a vertex structure for UI rendering,
//! a function to create a quad mesh, and a bitmap font structure for rendering text.

use std::sync::Arc;

use crate::{abs::Vertex, mesh::Mesh, shader::ShaderProgram, texture::Texture};
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
    pub fn text_metrics(&self, text: &str, font_size: f32) -> Vec2 {
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

        vec2(max_width, total_height)
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

/// NineSlice structure used for creating 9slice meshes from a specified texture region
pub struct NineSlice {
    pub position: Vec2,
    pub size: Vec2,

    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,

    pub uv_top_left: UVec2,
    pub uv_size: UVec2,

    /// Controls the scaling of the corners and edges without affecting the UVs
    pub scale: u32,

    pub atlas_size: Vec2,
}

impl NineSlice {
    pub fn build(&self, gl: &Arc<glow::Context>) -> Mesh {
        let left = self.left * self.scale;
        let right = self.right * self.scale;
        let top = self.top * self.scale;
        let bottom = self.bottom * self.scale;

        let x0 = self.position.x;
        let x1 = self.position.x + left as f32;
        let x2 = self.position.x + self.size.x - right as f32;
        let x3 = self.position.x + self.size.x;

        let y0 = self.position.y;
        let y1 = self.position.y + top as f32;
        let y2 = self.position.y + self.size.y - bottom as f32;
        let y3 = self.position.y + self.size.y;

        let uv_min = self.uv_top_left;
        let uv_max = self.uv_size + self.uv_top_left;

        let u0 = uv_min.x;
        let u3 = uv_max.x;

        let v0 = uv_min.y;
        let v3 = uv_max.y;

        let u1 = u0 + self.left;
        let u2 = u3 - self.right;

        let v1 = v0 + self.top;
        let v2 = v3 - self.bottom;

        let inv_atlas = 1.0 / self.atlas_size;
        let u0 = u0 as f32 * inv_atlas.x;
        let u1 = u1 as f32 * inv_atlas.x;
        let u2 = u2 as f32 * inv_atlas.x;
        let u3 = u3 as f32 * inv_atlas.x;
        let v0 = v0 as f32 * inv_atlas.y;
        let v1 = v1 as f32 * inv_atlas.y;
        let v2 = v2 as f32 * inv_atlas.y;
        let v3 = v3 as f32 * inv_atlas.y;

        #[rustfmt::skip]
        let positions = [
            vec2(x0, y0), vec2(x1, y0), vec2(x2, y0), vec2(x3, y0),
            vec2(x0, y1), vec2(x1, y1), vec2(x2, y1), vec2(x3, y1),
            vec2(x0, y2), vec2(x1, y2), vec2(x2, y2), vec2(x3, y2),
            vec2(x0, y3), vec2(x1, y3), vec2(x2, y3), vec2(x3, y3),
        ];

        #[rustfmt::skip]
        let uvs = [
            vec2(u0, v0), vec2(u1, v0), vec2(u2, v0), vec2(u3, v0),
            vec2(u0, v1), vec2(u1, v1), vec2(u2, v1), vec2(u3, v1),
            vec2(u0, v2), vec2(u1, v2), vec2(u2, v2), vec2(u3, v2),
            vec2(u0, v3), vec2(u1, v3), vec2(u2, v3), vec2(u3, v3),
        ];

        fn quad(i0: usize, i1: usize, i2: usize, i3: usize) -> [u32; 6] {
            [
                i0 as u32, i1 as u32, i2 as u32, i0 as u32, i2 as u32, i3 as u32,
            ]
        }

        let mut vertices = Vec::with_capacity(9 * 4);
        let mut indices = Vec::with_capacity(9 * 6);

        for row in 0..3 {
            for col in 0..3 {
                let i0 = row * 4 + col;
                let i1 = row * 4 + (col + 1);
                let i2 = (row + 1) * 4 + (col + 1);
                let i3 = (row + 1) * 4 + col;

                vertices.push(UIVertex {
                    position: vec3(positions[i1].x, positions[i1].y, 0.0),
                    uv: uvs[i1],
                });
                vertices.push(UIVertex {
                    position: vec3(positions[i0].x, positions[i0].y, 0.0),
                    uv: uvs[i0],
                });
                vertices.push(UIVertex {
                    position: vec3(positions[i3].x, positions[i3].y, 0.0),
                    uv: uvs[i3],
                });
                vertices.push(UIVertex {
                    position: vec3(positions[i2].x, positions[i2].y, 0.0),
                    uv: uvs[i2],
                });

                let base_index = ((row * 3) + col) * 4;
                indices.extend_from_slice(&quad(
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index + 3,
                ));
            }
        }

        Mesh::new(gl, &vertices, &indices, glow::TRIANGLES)
    }
}

/// Button structure for buttons that can be clicked by the user
pub struct Button {
    pub text: String,
    pub position: Vec2,
    pub size: Vec2,
    pub font_size: f32,
    pub disabled: bool,
    pressed: bool,
    pressed_last: bool,
    nineslice: NineSlice,
    bitmap_font: Arc<BitmapFont>,
}

impl Button {
    /// Creates a new button
    pub fn new(
        bitmap_font: &Arc<BitmapFont>,
        text: String,
        position: Vec2,
        size: Vec2,
        font_size: f32,
        disabled: bool,
    ) -> Self {
        Button {
            text,
            position,
            size,
            font_size,
            disabled,
            pressed: false,
            pressed_last: false,
            nineslice: NineSlice {
                position,
                size,
                left: 3,
                right: 3,
                top: 2,
                bottom: 7,
                uv_top_left: if !disabled { uvec2(0, 0) } else { uvec2(7, 0) },
                uv_size: uvec2(7, 10),
                scale: 4,
                atlas_size: vec2(144.0, 144.0),
            },
            bitmap_font: Arc::clone(bitmap_font),
        }
    }

    /// Updates the button's state
    pub fn update(&mut self, mouse: (Vec2, bool), grabbed: bool) {
        self.pressed_last = self.pressed;

        if self.disabled {
            self.nineslice.uv_top_left = uvec2(7, 0);
        } else {
            self.nineslice.uv_top_left = uvec2(0, 0);
        }

        if self.disabled || !mouse.1 || grabbed {
            self.pressed = false;
            return;
        }

        if mouse.0.x >= self.position.x
            && mouse.0.x <= self.position.x + self.size.x
            && mouse.0.y >= self.position.y
            && mouse.0.y <= self.position.y + self.size.y
        {
            self.pressed = true;
        } else {
            self.pressed = false;
        }
    }

    /// Returns true if the button was just pressed
    #[inline(always)]
    pub fn pressed(&self) -> bool {
        self.pressed && !self.pressed_last
    }

    /// Returns true if the button is down
    #[inline(always)]
    pub fn down(&self) -> bool {
        self.pressed
    }

    /// Returns true if the button was just released
    #[inline(always)]
    pub fn released(&self) -> bool {
        !self.pressed && self.pressed_last
    }

    /// Builds the meshes for the button (background and text)
    pub fn build_meshes(&self, gl: &Arc<glow::Context>) -> [Mesh; 2] {
        let text_metrics = self.bitmap_font.text_metrics(&self.text, self.font_size);
        let text_x = self.position.x + (self.size.x - text_metrics.x) * 0.5;
        let text_y = self.position.y + (self.size.y - text_metrics.y) * 0.5;
        let text = self
            .bitmap_font
            .build(gl, &self.text, text_x, text_y, self.font_size);
        let bg_mesh = self.nineslice.build(gl);
        [bg_mesh, text]
    }

    /// Draws the button on the screen
    pub fn draw(
        &self,
        gl: &Arc<glow::Context>,
        font_tex: &Texture,
        gui_tex: &Texture,
        ui_shader: &ShaderProgram,
    ) {
        let meshes = self.build_meshes(gl);
        gui_tex.bind_to_unit(0);
        ui_shader.set_uniform("ui_color", Vec4::ONE);
        meshes[0].draw();
        font_tex.bind_to_unit(0);
        meshes[1].draw();
    }
}
