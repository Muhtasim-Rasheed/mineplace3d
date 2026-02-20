use std::rc::Rc;

use glam::{Vec2, Vec4};

use crate::{
    abs::TextureHandle,
    render::ui::widgets::{Font, Label, NineSlice, Stack, Widget},
};

pub struct Button {
    pub position: Vec2,
    pub size: Vec2,
    pub label: String,
    pub label_color: Vec4,
    pub label_font_size: f32,
    is_down: bool,
    is_down_last: bool,
    hovered: bool,
    hover_last: bool,
    pub disabled: bool,
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
            hovered: false,
            hover_last: false,
            stack,
            texture,
            disabled: false,
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
            if self.hovered && !self.is_down {
                Vec4::ONE * 1.2
            } else {
                Vec4::ONE
            },
        ));
        self.stack.add_widget(Label::new(
            &self.label,
            self.label_font_size,
            self.label_color,
            &self.font,
        ));
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
            nine_slice.tint = if self.hovered && !self.is_down && !self.disabled {
                Vec4::ONE * 1.2
            } else {
                Vec4::ONE
            };
            nine_slice.position = self.position;
            nine_slice.size = self.size;
        } else {
            self.setup_stack();
        }
        if let Some(label) = self.stack.get_widget_mut::<Label>(1) {
            label.text = self.label.clone();
            label.color = self.label_color;
            label.font_size = self.label_font_size;
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

    fn size_hint(&self) -> Vec2 {
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
        let measured_size = self.size_hint().min(ctx.max_size);
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
