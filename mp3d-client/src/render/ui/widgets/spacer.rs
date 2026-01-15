use glam::Vec2;

use crate::render::ui::widgets::Widget;

pub struct Spacer {
    pub size: Vec2,
}

impl Spacer {
    pub fn new(size: Vec2) -> Self {
        Self { size }
    }
}

impl Widget for Spacer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        self.size
    }

    fn update(&mut self, _ctx: &super::UpdateContext) {
        // Spacer is static; no update logic needed.
    }

    fn layout(&mut self, _ctx: &super::LayoutContext) -> Vec2 {
        self.size_hint()
    }

    fn draw(&self, _ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        // Spacer has no visual representation; nothing to draw.
    }
}
