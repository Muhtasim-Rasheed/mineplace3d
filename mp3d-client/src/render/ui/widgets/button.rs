use std::rc::Rc;

use glam::{Vec2, Vec4};

use crate::{
    abs::TextureHandle,
    render::ui::widgets::{Font, Label, NineSlice, Stack, Widget},
};

pub struct Button {
    position: Vec2,
    size: Vec2,
    label: String,
    label_color: Vec4,
    label_font_size: f32,
    is_down: bool,
    is_down_last: bool,
    stack: Stack,
    texture: TextureHandle,
    font: Rc<Font>,
}

impl Button {
    pub fn new(
        label: &str,
        label_color: Vec4,
        label_font_size: f32,
        size: Vec2,
        font: &Rc<Font>,
        texture: TextureHandle,
    ) -> Self {
        let stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 0.0);
        let mut button = Self {
            position: Vec2::ZERO,
            size,
            label: label.to_string(),
            label_color,
            label_font_size,
            is_down: false,
            is_down_last: false,
            stack,
            texture,
            font: Rc::clone(font),
        };

        button.setup_stack();

        button
    }

    fn setup_stack(&mut self) {
        self.stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 0.0);
        self.stack.add_widget(NineSlice::new(
            self.texture,
            if self.is_down {
                glam::uvec2(16, 0)
            } else {
                glam::uvec2(0, 0)
            },
            glam::uvec2(16, 16),
            self.size,
            if self.is_down {
                glam::uvec4(5, 5, 6, 4)
            } else {
                glam::uvec4(5, 5, 4, 6)
            },
            4,
        ));
        self.stack.add_widget(Label::new(
            self.label.clone(),
            self.label_font_size,
            self.label_color,
            &self.font,
        ));
    }

    pub fn is_down(&self) -> bool {
        self.is_down
    }

    pub fn is_pressed(&self) -> bool {
        self.is_down && !self.is_down_last
    }

    pub fn is_released(&self) -> bool {
        !self.is_down && self.is_down_last
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = label.to_string();
        self.setup_stack();
    }

    pub fn set_size(&mut self, size: Vec2) {
        self.size = size;
        self.setup_stack();
    }
}

impl Widget for Button {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        self.size
    }

    fn update(&mut self, ctx: &super::UpdateContext) {
        self.is_down_last = self.is_down;
        let mouse_pos = ctx.mouse.position;
        let mouse_pressed = ctx.mouse.down.contains(&sdl2::mouse::MouseButton::Left);
        self.is_down = mouse_pressed
            && mouse_pos.x >= self.position.x
            && mouse_pos.x <= self.position.x + self.size.x
            && mouse_pos.y >= self.position.y
            && mouse_pos.y <= self.position.y + self.size.y;
        if self.is_pressed() || self.is_released() {
            self.setup_stack();
        }
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint();
        self.position = ctx.cursor;
        let layout_ctx = super::LayoutContext {
            max_size: measured_size,
            cursor: self.position,
        };
        self.stack.layout(&layout_ctx);
        Vec2::new(
            measured_size.x.min(ctx.max_size.x),
            measured_size.y.min(ctx.max_size.y),
        )
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        self.stack.draw(ui_renderer);
    }
}
