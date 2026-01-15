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

    /// Gets a certain widget by index.
    pub fn get_widget<T: Widget + 'static>(&self, index: usize) -> Option<&T> {
        self.widgets.get(index)?.as_any().downcast_ref::<T>()
    }

    /// Gets a certain widget by index as mutable.
    pub fn get_widget_mut<T: Widget + 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.widgets
            .get_mut(index)?.as_any_mut()
            .downcast_mut::<T>()
    }
}

impl Widget for Column {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

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

pub struct Stack {
    pub widgets: Vec<Box<dyn super::Widget>>,
    pub align_x: Alignment,
    pub align_y: Alignment,
    pub padding: f32,
}

impl Stack {
    /// Creates a new `Stack` container.
    pub fn new(align_x: Alignment, align_y: Alignment, padding: f32) -> Self {
        Self {
            widgets: Vec::new(),
            align_x,
            align_y,
            padding,
        }
    }

    /// Adds a widget to the stack.
    pub fn add_widget<T: Widget + 'static>(&mut self, widget: T) {
        self.widgets.push(Box::new(widget));
    }

    /// Gets a certain widget by index.
    pub fn get_widget<T: Widget + 'static>(&self, index: usize) -> Option<&T> {
        self.widgets.get(index)?.as_any().downcast_ref::<T>()
    }

    /// Gets a certain widget by index as mutable.
    pub fn get_widget_mut<T: Widget + 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.widgets
            .get_mut(index)?.as_any_mut()
            .downcast_mut::<T>()
    }
}

impl Widget for Stack {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> glam::Vec2 {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for widget in &self.widgets {
            let size = widget.size_hint();
            width = width.max(size.x);
            height = height.max(size.y);
        }

        width += self.padding * 2.0;
        height += self.padding * 2.0;

        glam::Vec2::new(width, height)
    }

    fn update(&mut self, ctx: &super::UpdateContext) {
        for widget in &mut self.widgets {
            widget.update(ctx);
        }
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> glam::Vec2 {
        let mut max_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;

        for widget in &mut self.widgets {
            let widget_size = widget.size_hint();
            let offset_x = match self.align_x {
                Alignment::Start => 0.0,
                Alignment::Center => (ctx.max_size.x - 2.0 * self.padding - widget_size.x) / 2.0,
                Alignment::End => ctx.max_size.x - self.padding - widget_size.x,
            };
            let offset_y = match self.align_y {
                Alignment::Start => 0.0,
                Alignment::Center => (ctx.max_size.y - 2.0 * self.padding - widget_size.y) / 2.0,
                Alignment::End => ctx.max_size.y - self.padding - widget_size.y,
            };

            let layout_ctx = super::LayoutContext {
                max_size: glam::Vec2::new(widget_size.x, widget_size.y),
                cursor: ctx.cursor + glam::Vec2::new(offset_x, offset_y),
            };

            let final_size = widget.layout(&layout_ctx);
            max_width = max_width.max(offset_x + final_size.x);
            max_height = max_height.max(offset_y + final_size.y);
        }

        glam::Vec2::new(max_width + self.padding, max_height + self.padding)
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        for widget in &self.widgets {
            widget.draw(ui_renderer);
        }
    }
}
