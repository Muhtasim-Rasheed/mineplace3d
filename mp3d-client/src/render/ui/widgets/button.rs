#![allow(dead_code)]

use glam::{Vec2, Vec4};

use crate::render::ui::widgets::{Label, NineSlice, Stack, Widget};

pub struct Button {
    position: Vec2,
    pub size: Vec2,
    pub text: String,
    pub color: Vec4,
    pub font_size: f32,
    pub always_hovered: bool,
    pub disabled: bool,
    is_down: bool,
    is_down_last: bool,
    hovered: bool,
    hover_last: bool,
    stack: Stack,
}

impl Button {
    pub fn new(text: &str) -> Self {
        let stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 0.0);
        let mut button = Self {
            position: Vec2::ZERO,
            size: Vec2::new(500.0, 80.0),
            text: text.to_string(),
            color: Vec4::ONE,
            font_size: 24.0,
            always_hovered: false,
            disabled: false,
            is_down: false,
            is_down_last: false,
            hovered: false,
            hover_last: false,
            stack,
        };

        button.setup_stack();

        button
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn always_hovered(mut self, always_hovered: bool) -> Self {
        self.always_hovered = always_hovered;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    fn setup_stack(&mut self) {
        self.stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 0.0)
            .with(NineSlice::new(
                [
                    if self.is_down {
                        glam::uvec2(16, 0)
                    } else {
                        glam::uvec2(0, 0)
                    },
                    glam::uvec2(16, 16),
                ],
                self.size,
                if self.is_down {
                    glam::uvec4(5, 5, 6, 4)
                } else {
                    glam::uvec4(5, 5, 4, 6)
                },
                4,
                0,
                if (self.hovered || self.always_hovered) && !self.is_down {
                    Vec4::ONE * 1.3
                } else {
                    Vec4::ONE
                },
            ))
            .with(
                Label::new(&self.text)
                    .font_size(self.font_size)
                    .color(self.color),
            );
    }

    fn update_stack(&mut self) {
        if let Some(nine_slice) = self.stack.get_widget_mut::<NineSlice>(0) {
            nine_slice.uv_top_left = if self.is_down || self.disabled {
                glam::uvec2(16, 0)
            } else {
                glam::uvec2(0, 0)
            };
            nine_slice.border = if self.is_down || self.disabled {
                glam::uvec4(5, 5, 6, 4)
            } else {
                glam::uvec4(5, 5, 4, 6)
            };
            nine_slice.tint =
                if (self.hovered || self.always_hovered) && !self.is_down && !self.disabled {
                    Vec4::ONE * 1.3
                } else {
                    Vec4::ONE
                };
            nine_slice.position = self.position;
            nine_slice.size = self.size;
        } else {
            self.setup_stack();
        }
        if let Some(label) = self.stack.get_widget_mut::<Label>(1) {
            label.text = self.text.clone();
            label.color = self.color;
            label.font_size = self.font_size;
        } else {
            self.setup_stack();
        }
    }

    pub fn is_down(&self) -> bool {
        self.is_down && !self.disabled
    }

    pub fn is_pressed(&self) -> bool {
        self.is_down && !self.is_down_last && !self.disabled
    }

    pub fn is_released(&self) -> bool {
        !self.is_down && self.is_down_last && !self.disabled
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }
}

impl Widget for Button {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self, _ctx: &super::LayoutContext) -> Vec2 {
        self.size
    }

    fn update(&mut self, ctx: &crate::other::UpdateContext) {
        self.is_down_last = self.is_down;
        self.hover_last = self.hovered;
        let mouse_pos = ctx.mouse.position;
        let mouse_pressed = ctx.mouse.down.contains(&sdl2::mouse::MouseButton::Left);
        self.hovered = mouse_pos.x >= self.position.x
            && mouse_pos.x <= self.position.x + self.size.x
            && mouse_pos.y >= self.position.y
            && mouse_pos.y <= self.position.y + self.size.y;
        self.is_down = mouse_pressed && self.hovered;
        self.update_stack();
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint(ctx).min(ctx.max_size);
        self.position = ctx.cursor;
        let layout_ctx = super::LayoutContext {
            max_size: measured_size,
            cursor: self.position,
            assets: ctx.assets,
        };
        self.stack.layout(&layout_ctx);
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
        self.stack.draw(ui_renderer, assets);
    }
}
