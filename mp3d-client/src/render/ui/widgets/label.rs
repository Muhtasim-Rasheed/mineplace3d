use glam::{Vec2, Vec4};
use mp3d_core::textcomponent::TextComponent;

use crate::{
    abs::Texture,
    render::ui::{uirenderer::DrawCommand, widgets::Widget},
    resource::fontsettings::FontSettings,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextParams {
    pub font_size: f32,
    pub color: Vec4,
    pub word_wrap_width: Option<f32>,
}

impl TextParams {
    pub fn without_color(self) -> ColorlessTextParams {
        ColorlessTextParams {
            font_size: self.font_size,
            word_wrap_width: self.word_wrap_width,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorlessTextParams {
    pub font_size: f32,
    pub word_wrap_width: Option<f32>,
}

impl ColorlessTextParams {
    pub fn with_color(self, color: Vec4) -> TextParams {
        TextParams {
            font_size: self.font_size,
            color,
            word_wrap_width: self.word_wrap_width,
        }
    }
}

impl Default for TextParams {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            color: Vec4::ONE,
            word_wrap_width: None,
        }
    }
}

impl Default for ColorlessTextParams {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            word_wrap_width: None,
        }
    }
}

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
            _ => {
                if let Some(i) = (c as u32).checked_sub(self.first_char as u32) {
                    Some(vec![i])
                } else {
                    None
                }
            }
        }
    }

    fn glyph_uvs(&self, c: char) -> Option<Vec<[Vec2; 2]>> {
        self.glyph_indices(c)
            .and_then(|indices| indices.into_iter().map(|i| self.index_to_uvs(i)).collect())
    }

    pub fn layout_text(&self, text: &str, params: ColorlessTextParams) -> Vec<(Vec2, char)> {
        let mut result = Vec::new();

        let mut cursor = Vec2::ZERO;
        let line_height = params.font_size;

        let wrap_width = params.word_wrap_width;

        for word in text.split_inclusive(|c: char| c.is_whitespace()) {
            let word_width: f32 = word
                .chars()
                .map(|c| self.char_width(params.font_size, c))
                .sum();

            if let Some(max_width) = wrap_width
                && cursor.x > 0.0
                && cursor.x + word_width > max_width
            {
                cursor.x = 0.0;
                cursor.y += line_height;
            }

            for c in word.chars() {
                if c == '\n' {
                    cursor.x = 0.0;
                    cursor.y += line_height;
                    continue;
                }

                result.push((cursor, c));
                cursor.x += self.char_width(params.font_size, c);
            }
        }

        result
    }

    pub fn measure_text(&self, text: &str, params: ColorlessTextParams) -> Vec2 {
        let layout = self.layout_text(text, params);

        let char_size = self.char_size(params.font_size);

        let mut max_x = 0.0_f32;
        let mut max_y = char_size.y;

        for (pos, c) in layout {
            max_x = max_x.max(pos.x + self.char_width(params.font_size, c));
            max_y = max_y.max(pos.y + char_size.y);
        }

        Vec2::new(max_x, max_y)
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

    fn char_width(&self, font_size: f32, c: char) -> f32 {
        let glyph_indices_len = self
            .glyph_indices(c)
            .map(|indices| indices.len())
            .unwrap_or(0);
        font_size * (self.char_size.x / self.char_size.y) * glyph_indices_len as f32
    }

    pub fn text(&self, text: &str, params: TextParams) -> Vec<DrawCommand> {
        let mut commands = Vec::new();

        for (pos, c) in self.layout_text(text, params.without_color()) {
            if let Some(uvs) = self.glyph_uvs(c) {
                let char_size = self.char_size(params.font_size);
                let pos = pos - Vec2::new(self.char_back(params.font_size, c), 0.0);

                for uv_rect in uvs {
                    let pos_min = pos;
                    let pos_max = pos + char_size;

                    commands.push(DrawCommand::Quad {
                        rect: [pos_min, pos_max],
                        uv_rect,
                        mode: crate::render::ui::uirenderer::UIRenderMode::Texture(
                            self.atlas().handle(),
                            params.color,
                        ),
                        layer: 2000,
                    });
                }
            }
        }

        commands
    }

    pub fn measure_component(
        &self,
        component: &TextComponent,
        params: ColorlessTextParams,
    ) -> Vec2 {
        let mut size = Vec2::ZERO;
        for part in &component.parts {
            let part_size = self.measure_text(&part.text, params);
            size.x += part_size.x;
            size.y = size.y.max(part_size.y);
        }
        size
    }

    pub fn text_component(
        &self,
        component: &TextComponent,
        params: ColorlessTextParams,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let mut cursor = Vec2::ZERO;
        for part in &component.parts {
            let part_commands = self.text(&part.text, params.with_color(part.color.into()));
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
            let part_size = self.measure_text(&part.text, params);
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
    pub wrap: Option<f32>,
}

impl Label {
    pub fn new(text: &str, font_size: f32, color: Vec4) -> Self {
        Self {
            text: text.to_string(),
            position: Vec2::ZERO,
            font_size,
            color,
            wrap: None,
        }
    }

    pub fn with_wrap(mut self, wrap_width: f32) -> Self {
        self.wrap = Some(wrap_width);
        self
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
        ctx.assets.font.measure_text(
            &self.text,
            ColorlessTextParams {
                font_size: self.font_size,
                word_wrap_width: self.wrap,
            },
        )
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
            .text(
                &self.text,
                TextParams {
                    font_size: self.font_size,
                    color: self.color,
                    word_wrap_width: self.wrap,
                },
            )
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
