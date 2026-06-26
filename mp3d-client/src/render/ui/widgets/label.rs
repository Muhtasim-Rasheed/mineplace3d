use glam::{Vec2, Vec4};

use crate::render::ui::{
    font::{ColorlessTextParams, TextParams},
    uirenderer::DrawCommand,
    widgets::Widget,
};

pub struct Label {
    pub text: String,
    position: Vec2,
    pub font_size: f32,
    pub color: Vec4,
    pub wrap: Option<f32>,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            position: Vec2::ZERO,
            font_size: 24.0,
            color: Vec4::ONE,
            wrap: None,
        }
    }

    pub fn color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn wrap(mut self, wrap_width: f32) -> Self {
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
