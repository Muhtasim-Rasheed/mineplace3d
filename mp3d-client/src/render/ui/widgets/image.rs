use glam::{UVec2, Vec2, Vec4};

use crate::{abs::TextureHandle, render::ui::widgets::Widget};

pub struct Image {
    pub uv_top_left: UVec2,
    pub uv_size: UVec2,
    position: Vec2,
    pub size: Vec2,
    pub tint: Vec4,
    pub layer: i32,
    pub tex_handle: TextureHandle,
}

impl Image {
    pub fn new(
        uvs: [UVec2; 2],
        size: Vec2,
        layer: i32,
        tint: Vec4,
        tex_handle: TextureHandle,
    ) -> Self {
        Image {
            uv_top_left: uvs[0],
            uv_size: uvs[1],
            position: Vec2::ZERO,
            size,
            tint,
            layer,
            tex_handle,
        }
    }
}

impl Widget for Image {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self, _ctx: &super::LayoutContext) -> Vec2 {
        self.size
    }

    fn update(&mut self, _ctx: &crate::other::UpdateContext) {
        // Image is static; no update logic needed.
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint(ctx);
        self.position = ctx.cursor;
        Vec2::new(
            measured_size.x.min(ctx.max_size.x),
            measured_size.y.min(ctx.max_size.y),
        )
    }

    fn draw(
        &self,
        ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer,
        _assets: &crate::scenes::Assets,
    ) {
        let gui_tex_size = glam::uvec2(self.tex_handle.width(), self.tex_handle.height()).as_vec2();
        let uv_min = self.uv_top_left.as_vec2() / gui_tex_size;
        let uv_max = (self.uv_top_left + self.uv_size).as_vec2() / gui_tex_size;
        ui_renderer.add_command(crate::render::ui::uirenderer::DrawCommand::Quad {
            rect: [self.position, self.position + self.size],
            uv_rect: [uv_min, uv_max],
            mode: crate::render::ui::uirenderer::UIRenderMode::Texture(self.tex_handle, self.tint),
            layer: self.layer,
        });
    }
}
