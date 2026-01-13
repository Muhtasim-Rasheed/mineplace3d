//! Containers that can hold multiple widgets.

use crate::render::ui::widgets::Widget;

/// Alignment options for widgets within a container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

/// A vertical column container that arranges its child widgets vertically.
pub struct Column {
    pub widgets: Vec<Box<dyn super::Widget>>,
    pub spacing: f32,
    pub alignment: Alignment,
    pub padding: f32,
}

impl Column {
    /// Creates a new `Column` container with the specified spacing and alignment.
    pub fn new(spacing: f32, alignment: Alignment, padding: f32) -> Self {
        Self {
            widgets: Vec::new(),
            spacing,
            alignment,
            padding,
        }
    }

    /// Adds a widget to the column.
    pub fn add_widget<T: Widget + 'static>(&mut self, widget: T) {
        self.widgets.push(Box::new(widget));
    }
}

impl Widget for Column {
    fn size_hint(&self) -> glam::Vec2 {
        let mut width: f32 = 0.0;
        let mut height = 0.0;

        for widget in &self.widgets {
            let size = widget.size_hint();
            width = width.max(size.x);
            height += size.y;
        }

        width += self.padding * 2.0;
        height += self.padding * 2.0;

        if !self.widgets.is_empty() {
            height += self.spacing * (self.widgets.len() as f32 - 1.0);
        }

        glam::Vec2::new(width, height)
    }

    fn update(&mut self, ctx: &super::UpdateContext) {
        for widget in &mut self.widgets {
            widget.update(ctx);
        }
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> glam::Vec2 {
        let mut cursor = ctx.cursor + glam::Vec2::new(self.padding, self.padding);
        let mut max_width: f32 = 0.0;

        for widget in &mut self.widgets {
            let widget_size = widget.size_hint();
            let offset_x = match self.alignment {
                Alignment::Start => 0.0,
                Alignment::Center => (ctx.max_size.x - 2.0 * self.padding - widget_size.x) / 2.0,
                Alignment::End => ctx.max_size.x * self.padding - widget_size.x,
            };

            let layout_ctx = super::LayoutContext {
                max_size: glam::Vec2::new(widget_size.x, ctx.max_size.y),
                cursor: cursor + glam::Vec2::new(offset_x, 0.0),
            };

            let final_size = widget.layout(&layout_ctx);
            max_width = max_width.max(offset_x + final_size.x);
            cursor.y += final_size.y + self.spacing;
        }

        cursor.y += self.padding - self.spacing;

        glam::Vec2::new(max_width, cursor.y - ctx.cursor.y)
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        for widget in &self.widgets {
            widget.draw(ui_renderer);
        }
    }
}
