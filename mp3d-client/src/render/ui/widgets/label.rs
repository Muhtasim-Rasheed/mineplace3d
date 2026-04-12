use glam::{Vec2, Vec4};
use mp3d_core::TextComponent;

use crate::{
    abs::Texture,
    render::ui::{uirenderer::DrawCommand, widgets::Widget},
    resource::fontsettings::FontSettings,
};

pub struct Font {
    atlas: Texture,
    char_size: Vec2,
    first_char: char,
    strikethrough: Option<u32>,
}

impl Font {
    pub fn new(atlas: Texture, font_settings: FontSettings) -> Self {
        Self {
            atlas,
            char_size: Vec2::new(
                font_settings.char_width as f32,
                font_settings.char_height as f32,
            ),
            first_char: font_settings.first_char,
            strikethrough: font_settings.strikethrough_idx,
        }
    }

    pub fn atlas(&self) -> &Texture {
        &self.atlas
    }

    fn index_to_uvs(&self, i: u32) -> Option<[Vec2; 2]> {
        let cols = self.atlas.width() / self.char_size.x as u32;
        let rows = self.atlas.height() / self.char_size.y as u32;

        if i < cols * rows {
            let col = i % cols;
            let row = i / cols;

            let uv_size = Vec2::new(1.0 / cols as f32, 1.0 / rows as f32);
            let uv_min = Vec2::new(col as f32 * uv_size.x, row as f32 * uv_size.y);
            let uv_max = uv_min + uv_size;

            Some([uv_min, uv_max])
        } else {
            None
        }
    }

    fn glyph_indices(&self, c: char) -> Option<Vec<u32>> {
        match c {
            '\u{0336}' => Some(vec![self.strikethrough?]),
            '\u{1F431}' => {
                let base = self.strikethrough?;
                Some(vec![base + 1, base + 2])
            }
            _ => Some(vec![c as u32 - self.first_char as u32]),
        }
    }

    fn glyph_uvs(&self, c: char) -> Option<Vec<[Vec2; 2]>> {
        self.glyph_indices(c)
            .and_then(|indices| indices.into_iter().map(|i| self.index_to_uvs(i)).collect())
    }

    pub fn measure_text(&self, text: &str, font_size: f32) -> Vec2 {
        let mut max_width = 0.0_f32;
        let mut line_width = 0.0;
        let mut line_count = 1;
        for c in text.chars() {
            if c == '\n' {
                max_width = max_width.max(line_width);
                line_width = 0.0;
                line_count += 1;
            } else if c != '\u{0336}' {
                let glyph_indices_len = self
                    .glyph_indices(c)
                    .map(|indices| indices.len())
                    .unwrap_or(0);
                line_width +=
                    font_size * (self.char_size.x / self.char_size.y) * glyph_indices_len as f32;
            }
        }
        max_width = max_width.max(line_width);
        Vec2::new(max_width, line_count as f32 * font_size)
    }

    fn char_back(&self, font_size: f32, c: char) -> f32 {
        match c {
            '\u{0336}' => font_size * (self.char_size.x / self.char_size.y),
            _ => 0.0,
        }
    }

    fn char_size(&self, font_size: f32) -> Vec2 {
        Vec2::new(font_size * (self.char_size.x / self.char_size.y), font_size)
    }

    pub fn text(&self, text: &str, font_size: f32, color: Vec4) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let mut cursor = Vec2::ZERO;
        let char_size = self.char_size(font_size);

        for line in text.lines() {
            for c in line.chars() {
                if let Some(uvs) = self.glyph_uvs(c) {
                    cursor.x -= self.char_back(font_size, c);

                    for uv_rect in uvs {
                        let pos_min = cursor;
                        let pos_max = cursor + char_size;

                        commands.push(DrawCommand::Quad {
                            rect: [pos_min, pos_max],
                            uv_rect,
                            mode: crate::render::ui::uirenderer::UIRenderMode::Texture(
                                self.atlas().handle(),
                                color,
                            ),
                            layer: 2000,
                        });
                        cursor.x += char_size.x;
                    }
                }
            }
            cursor.x = 0.0;
            cursor.y += char_size.y;
        }

        commands
    }

    pub fn measure_component(&self, component: &TextComponent, font_size: f32) -> Vec2 {
        let mut size = Vec2::ZERO;
        for part in &component.parts {
            let part_size = self.measure_text(&part.text, font_size);
            size.x += part_size.x;
            size.y = size.y.max(part_size.y);
        }
        size
    }

    pub fn text_component(&self, component: &TextComponent, font_size: f32) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let mut cursor = Vec2::ZERO;
        for part in &component.parts {
            let part_commands = self.text(&part.text, font_size, part.color.into());
            for mut cmd in part_commands {
                if let DrawCommand::Quad { rect, .. } = &mut cmd {
                    rect[0] += cursor;
                    rect[1] += cursor;
                } else if let DrawCommand::Mesh { vertices, .. } = &mut cmd {
                    for vertex in vertices {
                        vertex.position += cursor.extend(0.0);
                    }
                }
                commands.push(cmd);
            }
            let part_size = self.measure_text(&part.text, font_size);
            cursor.x += part_size.x;
        }
        commands
    }
}

pub struct Label {
    pub text: String,
    pub position: Vec2,
    pub font_size: f32,
    pub color: Vec4,
}

impl Label {
    pub fn new(text: &str, font_size: f32, color: Vec4) -> Self {
        Self {
            text: text.to_string(),
            position: Vec2::ZERO,
            font_size,
            color,
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

    fn size_hint(&self, ctx: &super::LayoutContext) -> Vec2 {
        ctx.assets.font.measure_text(&self.text, self.font_size)
    }

    fn update(&mut self, _ctx: &crate::other::UpdateContext) {
        // Labels are static; no update logic needed.
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
        assets: &crate::scenes::Assets,
    ) {
        let commands = assets
            .font
            .text(&self.text, self.font_size, self.color)
            .into_iter()
            .map(|mut cmd| {
                if let DrawCommand::Quad { rect, .. } = &mut cmd {
                    rect[0] += self.position;
                    rect[1] += self.position;
                } else if let DrawCommand::Mesh { vertices, .. } = &mut cmd {
                    for vertex in vertices {
                        vertex.position += self.position.extend(0.0);
                    }
                }
                cmd
            });

        for command in commands {
            ui_renderer.add_command(command);
        }

        ui_renderer.finish();
    }
}
