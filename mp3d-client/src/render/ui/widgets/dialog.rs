use glam::{UVec2, UVec4, Vec2, Vec4};

use crate::render::ui::widgets::{Label, NineSlice, Stack, Widget};

pub struct Dialog {
    position: Vec2,
    pub width: f32,
    pub label_text: String,
    pub label_color: Vec4,
    pub label_font_size: f32,

    stack: Stack,
}

impl Dialog {
    pub fn new(label_text: &str, label_color: Vec4, label_font_size: f32, width: f32) -> Self {
        let stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 0.0);
        let mut dialog = Self {
            position: Vec2::ZERO,
            width,
            label_text: label_text.to_string(),
            label_color,
            label_font_size,
            stack,
        };

        dialog.setup_stack();

        dialog
    }

    fn setup_stack(&mut self) {
        self.stack = Stack::new(super::Alignment::Center, super::Alignment::Center, 8.0);
        self.stack.add_widget(NineSlice::new(
            [UVec2::new(48, 16), UVec2::new(16, 16)],
            Vec2::new(self.width, 0.0),
            UVec4::new(3, 3, 3, 5),
            4,
            0,
            Vec4::ONE,
        ));
        self.stack.add_widget(
            Label::new(&self.label_text, self.label_font_size, self.label_color)
                .with_wrap(self.width - 16.0 * 2.0),
        );
    }

    fn layout_stack(&mut self, layout_ctx: &super::LayoutContext) {
        let mut label_size = Vec2::ZERO;
        if let Some(label) = self.stack.get_widget_mut::<Label>(1) {
            label.text = self.label_text.clone();
            label.color = self.label_color;
            label.font_size = self.label_font_size;
            label_size = label.size_hint(layout_ctx);
        } else {
            self.setup_stack();
        }
        if let Some(nineslice) = self.stack.get_widget_mut::<NineSlice>(0) {
            nineslice.size.y = label_size.y + 16.0 + 32.0;
        }
    }
}

impl Widget for Dialog {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self, ctx: &super::LayoutContext) -> Vec2 {
        let label_size = if let Some(label) = self.stack.get_widget::<Label>(1) {
            label.size_hint(ctx)
        } else {
            Vec2::ZERO
        };
        Vec2::new(self.width, label_size.y + 16.0 + 32.0)
    }

    fn update(&mut self, ctx: &crate::other::UpdateContext) {
        self.stack.update(ctx);
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint(ctx).min(ctx.max_size);
        self.position = ctx.cursor;
        let layout_ctx = super::LayoutContext {
            max_size: measured_size,
            cursor: self.position,
            assets: ctx.assets,
        };
        self.layout_stack(&layout_ctx);
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
