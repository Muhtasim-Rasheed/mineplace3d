use crate::mesh::{DrawMode, Mesh, UIVertex};
use glam::*;
use image::DynamicImage;

pub struct BitmapFont {
    first_char: char,
    chars_per_row: u32,
    char_width: u32,
    char_height: u32,
    atlas: DynamicImage,
}

impl BitmapFont {
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

    pub fn build(&self, text: &str, start_x: f32, start_y: f32, font_size: f32) -> Mesh<UIVertex> {
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

        Mesh::new(&vertices, &indices, DrawMode::Triangles)
    }
}
