//! Contains all widgets and the `Widget` trait for building user interfaces.

use glam::Vec2;

use super::uirenderer::UIRenderer;

/// Context provided to widgets during the layout phase.
pub struct LayoutContext {
    pub max_size: Vec2,
    pub cursor: Vec2,
}

/// A widget trait for building user interfaces.
pub trait Widget {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Gives a hint of the desired size of the widget.
    fn size_hint(&self) -> Vec2 {
        Vec2::ZERO
    }

    /// Updates the widget state.
    fn update(&mut self, ctx: &crate::other::UpdateContext);

    /// Updates the widget layout given the available space.
    fn layout(&mut self, ctx: &LayoutContext) -> Vec2;

    /// Draws the widget with the given UI renderer.
    fn draw(&self, ui_renderer: &mut UIRenderer);
}

pub mod button;
pub mod containers;
pub mod label;
pub mod nineslice;

pub use button::*;
pub use containers::*;
pub use label::*;
pub use nineslice::*;
