#![allow(dead_code)]

use glam::{Vec2, Vec4};

use crate::render::ui::widgets::{Label, NineSlice, Stack, Widget};

/// Width : Height ratio of the knob.
const KNOB_ASP_RATIO: f32 = 2.0 / 5.0;

pub struct Slider {
    pub position: Vec2,
    pub size: Vec2,
    pub value: f32,
    pub label: String,
    pub label_color: Vec4,
    pub label_font_size: f32,
    pub min_value: f32,
    pub max_value: f32,
    is_dragging: bool,
    hovered: bool,
    stack: Stack,
    knob: NineSlice,
}

impl Slider {
    pub fn new(
        label: &str,
        label_color: Vec4,
        label_font_size: f32,
        size: Vec2,
        range: std::ops::RangeInclusive<f32>,
    ) -> Self {
        let stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 0.0);
        let knob_size_y = size.y * 1.2;
        let knob = NineSlice::new(
            [glam::uvec2(48, 0), glam::uvec2(8, 16)],
            Vec2::new(knob_size_y * KNOB_ASP_RATIO, knob_size_y),
            glam::uvec4(2, 2, 2, 2),
            4,
            0,
            Vec4::ONE,
        );
        let mut slider = Self {
            position: Vec2::ZERO,
            size,
            value: 0.0,
            label: label.to_string(),
            label_color,
            label_font_size,
            min_value: *range.start(),
            max_value: *range.end(),
            is_dragging: false,
            hovered: false,
            stack,
            knob,
        };

        slider.setup_widgets();

        slider
    }

    fn value_normalized(&self) -> f32 {
        (self.value - self.min_value) / (self.max_value - self.min_value)
    }

    fn label_text(&self) -> String {
        format!("{}: {:.0}%", self.label, self.value * 100.0)
    }

    fn knob_position(&self) -> Vec2 {
        let min_x = self.position.x + 32.0;
        let max_x = self.position.x + self.size.x - 32.0;
        Vec2::new(
            min_x + (max_x - min_x) * self.value_normalized() - self.knob.size.x / 2.0,
            self.position.y + self.size.y / 2.0 - self.knob.size.y / 2.0,
        )
    }

    fn setup_widgets(&mut self) {
        self.stack.add_widget(NineSlice::new(
            [glam::uvec2(32, 0), glam::uvec2(16, 16)],
            self.size,
            glam::uvec4(6, 6, 4, 4),
            4,
            0,
            if self.hovered && !self.is_dragging {
                Vec4::new(1.2, 1.2, 1.2, 1.0)
            } else {
                Vec4::ONE
            },
        ));
        self.stack.add_widget(Label::new(
            &self.label_text(),
            self.label_font_size,
            self.label_color,
        ));
        self.knob.position = self.knob_position();
    }

    fn update_widgets(&mut self) {
        self.stack.get_widget_mut::<Label>(1).unwrap().text = self.label_text();
        self.knob.position = self.knob_position();
    }
}

impl Widget for Slider {
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
        let mouse_pos = ctx.mouse.position;
        self.hovered = mouse_pos.x >= self.position.x
            && mouse_pos.x <= self.position.x + self.size.x
            && mouse_pos.y >= self.position.y
            && mouse_pos.y <= self.position.y + self.size.y;

        if ctx.mouse.down.contains(&sdl2::mouse::MouseButton::Left)
            && (self.hovered || self.is_dragging)
        {
            self.is_dragging = true;
            let relative_mouse_x = (mouse_pos.x - self.position.x).clamp(0.0, self.size.x);
            // self.value = relative_mouse_x / self.size.x;
            self.value = self.min_value
                + (self.max_value - self.min_value) * (relative_mouse_x / self.size.x);
        } else {
            self.is_dragging = false;
        }
        self.update_widgets();
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint(ctx);
        self.position = ctx.cursor;
        self.stack.layout(&super::LayoutContext {
            max_size: measured_size,
            cursor: self.position,
            assets: ctx.assets,
        });
        self.knob.layout(&super::LayoutContext {
            max_size: measured_size,
            cursor: self.knob_position(),
            assets: ctx.assets,
        });
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
        self.knob.draw(ui_renderer, assets);
    }
}
