use glam::{UVec2, UVec4, Vec2, Vec4, vec2};

use crate::{abs::TextureHandle, render::ui::widgets::Widget};

pub struct NineSlice {
    pub texture: TextureHandle,
    pub uv_top_left: UVec2,
    pub uv_size: UVec2,
    pub position: Vec2,
    pub size: Vec2,
    pub border: UVec4,
    /// Scales the borders without changing the overall size of the nine-slice and the UVs.
    pub scale: u32,
    pub tint: Vec4,
    atlas_size: UVec2,
}

impl NineSlice {
    pub fn new(
        texture: TextureHandle,
        uv_top_left: UVec2,
        uv_size: UVec2,
        size: Vec2,
        border: UVec4,
        scale: u32,
        tint: Vec4,
    ) -> Self {
        Self {
            texture,
            uv_top_left,
            uv_size,
            position: Vec2::ZERO,
            size,
            border,
            scale,
            tint,
            atlas_size: UVec2::new(texture.width(), texture.height()),
        }
    }
}

impl Widget for NineSlice {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        self.size
    }

    fn update(&mut self, _ctx: &crate::other::UpdateContext) {
        // NineSlice is static; no update logic needed.
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint();
        self.position = ctx.cursor;
        Vec2::new(
            measured_size.x.min(ctx.max_size.x),
            measured_size.y.min(ctx.max_size.y),
        )
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        let left = self.border.x * self.scale;
        let right = self.border.y * self.scale;
        let top = self.border.z * self.scale;
        let bottom = self.border.w * self.scale;

        let x0 = self.position.x;
        let x1 = self.position.x + left as f32;
        let x2 = self.position.x + self.size.x - right as f32;
        let x3 = self.position.x + self.size.x;

        let y0 = self.position.y;
        let y1 = self.position.y + top as f32;
        let y2 = self.position.y + self.size.y - bottom as f32;
        let y3 = self.position.y + self.size.y;

        let uv_min = self.uv_top_left;
        let uv_max = self.uv_top_left + self.uv_size;

        let inv_atlas = 1.0 / self.atlas_size.as_vec2();

        let u0 = uv_min.x as f32;
        let u3 = uv_max.x as f32;
        let v0 = uv_min.y as f32;
        let v3 = uv_max.y as f32;
        let u1 = u0 + self.border.x as f32;
        let u2 = u3 - self.border.y as f32;
        let v1 = v0 + self.border.z as f32;
        let v2 = v3 - self.border.w as f32;
        let [u0, u1, u2, u3] = [u0, u1, u2, u3].map(|u| u * inv_atlas.x);
        let [v0, v1, v2, v3] = [v0, v1, v2, v3].map(|v| v * inv_atlas.y);

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

        for row in 0..3 {
            for col in 0..3 {
                let pos_min = positions[row * 4 + col];
                let pos_max = positions[(row + 1) * 4 + (col + 1)];
                let uv_min = uvs[row * 4 + col];
                let uv_max = uvs[(row + 1) * 4 + (col + 1)];

                ui_renderer.add_command(crate::render::ui::uirenderer::DrawCommand {
                    rect: [pos_min, pos_max],
                    uv_rect: [uv_min, uv_max],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Texture(
                        self.texture,
                        self.tint,
                    ),
                });
            }
        }

        ui_renderer.finish();
    }
}
