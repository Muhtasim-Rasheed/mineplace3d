use std::rc::Rc;

use glam::{Vec2, Vec4};

use crate::{
    abs::Texture,
    render::ui::{uirenderer::DrawCommand, widgets::Widget},
};

pub struct Font {
    atlas: Texture,
    char_size: Vec2,
    first_char: char,
}

impl Font {
    pub fn new(atlas: Texture, char_size: Vec2, first_char: char) -> Self {
        Self {
            atlas,
            char_size,
            first_char,
        }
    }

    pub fn atlas(&self) -> &Texture {
        &self.atlas
    }

    pub fn glyph_uvs(&self, c: char) -> Option<[Vec2; 2]> {
        let index = c as u32 - self.first_char as u32;
        let cols = self.atlas.width() / self.char_size.x as u32;
        let rows = self.atlas.height() / self.char_size.y as u32;

        if index < cols * rows {
            let col = index % cols;
            let row = index / cols;

            let uv_size = Vec2::new(1.0 / cols as f32, 1.0 / rows as f32);
            let uv_min = Vec2::new(col as f32 * uv_size.x, row as f32 * uv_size.y);
            let uv_max = uv_min + uv_size;

            Some([uv_min, uv_max])
        } else {
            None
        }
    }

    pub fn measure_text(&self, text: &str, font_size: f32) -> Vec2 {
        let lines: Vec<&str> = text.split('\n').collect();
        let line_height = font_size;
        let max_width = lines
            .iter()
            .map(|line| line.len() as f32 * font_size * (self.char_size.x / self.char_size.y))
            .fold(0.0, f32::max);
        Vec2::new(max_width, lines.len() as f32 * line_height)
    }

    pub fn char_size(&self, font_size: f32) -> Vec2 {
        Vec2::new(font_size * (self.char_size.x / self.char_size.y), font_size)
    }

    pub fn text(&self, text: &str, font_size: f32, color: Vec4) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let mut cursor = Vec2::ZERO;
        let char_size = self.char_size(font_size);

        for line in text.lines() {
            for c in line.chars() {
                if let Some(uvs) = self.glyph_uvs(c) {
                    let pos_min = cursor;
                    let pos_max = cursor + char_size;

                    commands.push(DrawCommand {
                        rect: [pos_min, pos_max],
                        uv_rect: uvs,
                        mode: crate::render::ui::uirenderer::UIRenderMode::Texture(
                            self.atlas().handle(),
                            color,
                        ),
                    });
                }
                cursor.x += char_size.x;
            }
            cursor.x = 0.0;
            cursor.y += char_size.y;
        }

        commands
    }
}

pub struct Label {
    pub text: String,
    pub position: Vec2,
    pub font_size: f32,
    pub color: Vec4,
    pub font: Rc<Font>,
}

impl Label {
    pub fn new(text: &str, font_size: f32, color: Vec4, font: &Rc<Font>) -> Self {
        Self {
            text: text.to_string(),
            position: Vec2::ZERO,
            font_size,
            color,
            font: Rc::clone(font),
        }
    }
}

impl Widget for Label {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        self.font.measure_text(&self.text, self.font_size)
    }

    fn update(&mut self, _ctx: &super::UpdateContext) {
        // Labels are static; no update logic needed.
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
        let commands = self
            .font
            .text(&self.text, self.font_size, self.color)
            .into_iter()
            .map(|mut cmd| {
                cmd.rect[0] += self.position;
                cmd.rect[1] += self.position;
                cmd
            });

        for command in commands {
            ui_renderer.add_command(command);
        }

        ui_renderer.finish();
    }
}
