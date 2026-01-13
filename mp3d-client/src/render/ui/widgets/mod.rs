//! Contains all widgets and the `Widget` trait for building user interfaces.

use std::collections::HashSet;

use glam::Vec2;
use sdl2::{keyboard::Keycode, mouse::MouseButton};

use super::uirenderer::UIRenderer;

/// The current state of the keyboard.
#[derive(Default)]
pub struct KeyboardState {
    pub down: HashSet<Keycode>,
    pub pressed: HashSet<Keycode>,
    pub released: HashSet<Keycode>,
    pub text_input: String,
}

/// The current state of the mouse.
#[derive(Default)]
pub struct MouseState {
    pub position: Vec2,
    pub delta: Vec2,
    pub down: HashSet<MouseButton>,
    pub pressed: HashSet<MouseButton>,
    pub released: HashSet<MouseButton>,
    pub scroll_delta: Vec2,
}

/// Context provided to widgets during the update phase.
pub struct UpdateContext<'a> {
    pub keyboard: &'a KeyboardState,
    pub mouse: &'a MouseState,
    pub delta_time: f32,
}

impl<'a> UpdateContext<'a> {
    /// Creates a new `UpdateContext` from the given keyboard and mouse states and delta time.
    pub fn new(keyboard: &'a KeyboardState, mouse: &'a MouseState, delta_time: f32) -> Self {
        Self {
            keyboard,
            mouse,
            delta_time,
        }
    }
}

/// Context provided to widgets during the layout phase.
pub struct LayoutContext {
    pub max_size: Vec2,
    pub cursor: Vec2,
}

/// A widget trait for building user interfaces.
pub trait Widget {
    /// Gives a hint of the desired size of the widget.
    fn size_hint(&self) -> Vec2 {
        Vec2::ZERO
    }

    /// Updates the widget state.
    fn update(&mut self, ctx: &UpdateContext);

    /// Updates the widget layout given the available space.
    fn layout(&mut self, ctx: &LayoutContext) -> Vec2;

    /// Draws the widget with the given UI renderer.
    fn draw(&self, ui_renderer: &mut UIRenderer);
}

pub mod label;
pub mod containers;

pub use label::*;
pub use containers::*;
