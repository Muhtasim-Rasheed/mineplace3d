//! Containers that can hold multiple widgets.

use glam::{Vec2, Vec4};

use crate::render::ui::widgets::Widget;

/// Alignment options for widgets within a container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

/// Justification options for widgets within a container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Justification {
    Start,
    Center,
    End,
    SpaceBetween,
}

/// A vertical column container that arranges its child widgets vertically.
pub struct Column {
    pub widgets: Vec<Box<dyn super::Widget>>,
    pub spacing: f32,
    pub alignment: Alignment,
    pub padding: Vec4,
    pub justification: Justification,
    pub min_size: Vec2,
}

impl Column {
    /// Creates a new `Column` container with the specified spacing, alignment, padding, and
    /// justification.
    pub fn new(
        spacing: f32,
        alignment: Alignment,
        padding: Vec4,
        justification: Justification,
    ) -> Self {
        Self {
            widgets: Vec::new(),
            spacing,
            alignment,
            padding,
            justification,
            min_size: Vec2::ZERO,
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
            .get_mut(index)?
            .as_any_mut()
            .downcast_mut::<T>()
    }

    /// Traverses through containers to find a widget of type T and returns a reference.
    pub fn find_widget<T: Widget + 'static>(&self, indices: &[usize]) -> Option<&T> {
        let mut current: &dyn Widget = self;
        for &index in indices {
            let container_any = current.as_any();
            match container_any.type_id() {
                id if id == std::any::TypeId::of::<Column>() => {
                    let container = container_any.downcast_ref::<Column>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                id if id == std::any::TypeId::of::<Row>() => {
                    let container = container_any.downcast_ref::<Row>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                id if id == std::any::TypeId::of::<Stack>() => {
                    let container = container_any.downcast_ref::<Stack>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                _ => return None,
            }
        }
        current.as_any().downcast_ref::<T>()
    }

    /// Traverses through containers to find a widget of type T and returns a mutable reference.
    pub fn find_widget_mut<T: Widget + 'static>(&mut self, indices: &[usize]) -> Option<&mut T> {
        let mut current: &mut dyn Widget = self;
        for &index in indices {
            let container_any = current.as_any_mut();
            match container_any.type_id() {
                id if id == std::any::TypeId::of::<Column>() => {
                    let container = container_any.downcast_mut::<Column>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                id if id == std::any::TypeId::of::<Row>() => {
                    let container = container_any.downcast_mut::<Row>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                id if id == std::any::TypeId::of::<Stack>() => {
                    let container = container_any.downcast_mut::<Stack>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                _ => return None,
            }
        }
        current.as_any_mut().downcast_mut::<T>()
    }
}

impl Widget for Column {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for widget in &self.widgets {
            let size = widget.size_hint();
            width = width.max(size.x);
            height += size.y;
        }

        height += self.spacing * (self.widgets.len().saturating_sub(1)) as f32;
        width += self.padding.x + self.padding.z;
        height += self.padding.y + self.padding.w;

        Vec2::new(width, height).max(self.min_size)
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let total_height_widget = self.widgets.iter().map(|w| w.size_hint().y).sum::<f32>();

        let mut total_height: f32 = 0.0;

        for widget in &self.widgets {
            total_height += widget.size_hint().y;
        }

        let spacing = match self.justification {
            Justification::SpaceBetween if self.widgets.len() > 1 => {
                let content_height = ctx.max_size.y - self.padding.z - self.padding.w;

                ((content_height - total_height_widget) / (self.widgets.len() as f32 - 1.0))
                    .max(0.0)
            }
            _ => self.spacing,
        };

        total_height += spacing * (self.widgets.len().saturating_sub(1)) as f32;

        let mut cursor_y = match self.justification {
            Justification::Start => ctx.cursor.y + self.padding.z,
            Justification::Center => {
                ctx.cursor.y + (ctx.max_size.y - total_height) / 2.0 + self.padding.z
            }
            Justification::End => ctx.cursor.y + ctx.max_size.y - total_height - self.padding.w,
            Justification::SpaceBetween => ctx.cursor.y + self.padding.z,
        };

        for widget in self.widgets.iter_mut() {
            let widget_size = widget.size_hint();
            let offset_x = match self.alignment {
                Alignment::Start => self.padding.x,
                Alignment::Center => (ctx.max_size.x - widget_size.x) / 2.0,
                Alignment::End => ctx.max_size.x - widget_size.x - self.padding.z,
            };

            let layout_ctx = super::LayoutContext {
                max_size: Vec2::new(widget_size.x, widget_size.y),
                cursor: Vec2::new(ctx.cursor.x + offset_x, cursor_y),
            };

            widget.layout(&layout_ctx);
            cursor_y += widget_size.y + spacing;
        }

        Vec2::new(
            ctx.max_size.x,
            total_height + self.padding.y + self.padding.w,
        )
    }

    fn update(&mut self, ctx: &super::UpdateContext) {
        for widget in &mut self.widgets {
            widget.update(ctx);
        }
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        for widget in &self.widgets {
            widget.draw(ui_renderer);
        }
    }
}

/// A horizontal row container that arranges its child widgets horizontally.
pub struct Row {
    pub widgets: Vec<Box<dyn super::Widget>>,
    pub spacing: f32,
    pub alignment: Alignment,
    pub padding: Vec4,
    pub justification: Justification,
    pub min_size: Vec2,
}

impl Row {
    /// Creates a new `Row` container with the specified spacing, alignment, padding, and
    /// justification.
    pub fn new(
        spacing: f32,
        alignment: Alignment,
        padding: Vec4,
        justification: Justification,
    ) -> Self {
        Self {
            widgets: Vec::new(),
            spacing,
            alignment,
            padding,
            justification,
            min_size: Vec2::ZERO,
        }
    }

    /// Adds a widget to the row.
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
            .get_mut(index)?
            .as_any_mut()
            .downcast_mut::<T>()
    }

    /// Traverses through containers to find a widget of type T and returns a reference.
    pub fn find_widget<T: Widget + 'static>(&self, indices: &[usize]) -> Option<&T> {
        let mut current: &dyn Widget = self;
        for &index in indices {
            let container_any = current.as_any();
            match container_any.type_id() {
                id if id == std::any::TypeId::of::<Column>() => {
                    let container = container_any.downcast_ref::<Column>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                id if id == std::any::TypeId::of::<Row>() => {
                    let container = container_any.downcast_ref::<Row>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                id if id == std::any::TypeId::of::<Stack>() => {
                    let container = container_any.downcast_ref::<Stack>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                _ => return None,
            }
        }
        current.as_any().downcast_ref::<T>()
    }

    /// Traverses through containers to find a widget of type T and returns a mutable reference.
    pub fn find_widget_mut<T: Widget + 'static>(&mut self, indices: &[usize]) -> Option<&mut T> {
        let mut current: &mut dyn Widget = self;
        for &index in indices {
            let container_any = current.as_any_mut();
            match container_any.type_id() {
                id if id == std::any::TypeId::of::<Column>() => {
                    let container = container_any.downcast_mut::<Column>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                id if id == std::any::TypeId::of::<Row>() => {
                    let container = container_any.downcast_mut::<Row>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                id if id == std::any::TypeId::of::<Stack>() => {
                    let container = container_any.downcast_mut::<Stack>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                _ => return None,
            }
        }
        current.as_any_mut().downcast_mut::<T>()
    }
}

impl Widget for Row {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for widget in &self.widgets {
            let size = widget.size_hint();
            width += size.x;
            height = height.max(size.y);
        }

        width += self.spacing * (self.widgets.len().saturating_sub(1)) as f32;
        width += self.padding.x + self.padding.z;
        height += self.padding.y + self.padding.w;

        Vec2::new(width, height).max(self.min_size)
    }

    fn update(&mut self, ctx: &super::UpdateContext) {
        for widget in &mut self.widgets {
            widget.update(ctx);
        }
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let total_width_widget = self.widgets.iter().map(|w| w.size_hint().x).sum::<f32>();

        let mut total_width: f32 = 0.0;

        for widget in &self.widgets {
            total_width += widget.size_hint().x;
        }

        let spacing = match self.justification {
            Justification::SpaceBetween if self.widgets.len() > 1 => {
                let content_width = ctx.max_size.x - self.padding.x - self.padding.y;

                ((content_width - total_width_widget) / (self.widgets.len() as f32 - 1.0)).max(0.0)
            }
            _ => self.spacing,
        };

        total_width += spacing * (self.widgets.len().saturating_sub(1)) as f32;

        let mut cursor_x = match self.justification {
            Justification::Start => ctx.cursor.x + self.padding.x,
            Justification::Center => {
                ctx.cursor.x + (ctx.max_size.x - total_width) / 2.0 + self.padding.x
            }
            Justification::End => ctx.cursor.x + ctx.max_size.x - total_width - self.padding.y,
            Justification::SpaceBetween => ctx.cursor.x + self.padding.x,
        };

        for widget in self.widgets.iter_mut() {
            let widget_size = widget.size_hint();
            let offset_y = match self.alignment {
                Alignment::Start => self.padding.z,
                Alignment::Center => (ctx.max_size.y - widget_size.y) / 2.0,
                Alignment::End => ctx.max_size.y - widget_size.y - self.padding.w,
            };

            let layout_ctx = super::LayoutContext {
                max_size: Vec2::new(widget_size.x, widget_size.y),
                cursor: Vec2::new(cursor_x, ctx.cursor.y + offset_y),
            };

            widget.layout(&layout_ctx);
            cursor_x += widget_size.x + spacing;
        }

        Vec2::new(
            total_width + self.padding.x + self.padding.z,
            ctx.max_size.y,
        )
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        for widget in &self.widgets {
            widget.draw(ui_renderer);
        }
    }
}

/// A stack container that overlays its child widgets on top of each other.
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
            .get_mut(index)?
            .as_any_mut()
            .downcast_mut::<T>()
    }

    /// Traverses through containers to find a widget of type T and returns a reference.
    pub fn find_widget<T: Widget + 'static>(&self, indices: &[usize]) -> Option<&T> {
        let mut current: &dyn Widget = self;
        for &index in indices {
            let container_any = current.as_any();
            match container_any.type_id() {
                id if id == std::any::TypeId::of::<Column>() => {
                    let container = container_any.downcast_ref::<Column>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                id if id == std::any::TypeId::of::<Row>() => {
                    let container = container_any.downcast_ref::<Row>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                id if id == std::any::TypeId::of::<Stack>() => {
                    let container = container_any.downcast_ref::<Stack>().unwrap();
                    current = container.widgets.get(index)?.as_ref();
                }
                _ => return None,
            }
        }
        current.as_any().downcast_ref::<T>()
    }

    /// Traverses through containers to find a widget of type T and returns a mutable reference.
    pub fn find_widget_mut<T: Widget + 'static>(&mut self, indices: &[usize]) -> Option<&mut T> {
        let mut current: &mut dyn Widget = self;
        for &index in indices {
            let container_any = current.as_any_mut();
            match container_any.type_id() {
                id if id == std::any::TypeId::of::<Column>() => {
                    let container = container_any.downcast_mut::<Column>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                id if id == std::any::TypeId::of::<Row>() => {
                    let container = container_any.downcast_mut::<Row>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                id if id == std::any::TypeId::of::<Stack>() => {
                    let container = container_any.downcast_mut::<Stack>().unwrap();
                    current = container.widgets.get_mut(index)?.as_mut();
                }
                _ => return None,
            }
        }
        current.as_any_mut().downcast_mut::<T>()
    }
}

impl Widget for Stack {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for widget in &self.widgets {
            let size = widget.size_hint();
            width = width.max(size.x);
            height = height.max(size.y);
        }

        width += self.padding * 2.0;
        height += self.padding * 2.0;

        Vec2::new(width, height)
    }

    fn update(&mut self, ctx: &super::UpdateContext) {
        for widget in &mut self.widgets {
            widget.update(ctx);
        }
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
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
                max_size: Vec2::new(widget_size.x, widget_size.y),
                cursor: ctx.cursor + Vec2::new(offset_x, offset_y),
            };

            let final_size = widget.layout(&layout_ctx);
            max_width = max_width.max(offset_x + final_size.x);
            max_height = max_height.max(offset_y + final_size.y);
        }

        Vec2::new(max_width + self.padding, max_height + self.padding)
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        for widget in &self.widgets {
            widget.draw(ui_renderer);
        }
    }
}
