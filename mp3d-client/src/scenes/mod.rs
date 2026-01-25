//! Module providing the `Scene` trait and all scene implementations.
//!
//! This module serves as a central point for managing different scenes in the game client.

use std::sync::Arc;

use crate::render::ui::uirenderer::UIRenderer;

pub enum SceneSwitch {
    None,
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
    Quit,
}

/// The Scene trait defines the common interface for all scenes in the game client.
pub trait Scene {
    /// Handles an event.
    fn handle_event(&mut self, _event: &sdl2::event::Event) {}

    /// Updates the scene state.
    fn update(
        &mut self,
        ctx: &crate::other::UpdateContext,
        window: &sdl2::video::Window,
    ) -> SceneSwitch {
        SceneSwitch::None
    }

    /// Renders the scene.
    fn render(&mut self, gl: &Arc<glow::Context>, ui: &mut UIRenderer);
}

/// Manages the stack of scenes.
pub struct SceneManager {
    scenes: Vec<Box<dyn Scene>>,
}

impl SceneManager {
    /// Creates a new SceneManager with the initial scene.
    pub fn new(initial_scene: Box<dyn Scene>) -> Self {
        Self {
            scenes: vec![initial_scene],
        }
    }

    /// Handles an event by passing it to the current scene.
    pub fn handle_event(&mut self, event: &sdl2::event::Event) {
        if let Some(current_scene) = self.scenes.last_mut() {
            current_scene.handle_event(event);
        }
    }

    /// Updates the current scene and manages scene transitions.
    pub fn update(
        &mut self,
        ctx: &crate::other::UpdateContext,
        window: &sdl2::video::Window,
    ) -> bool {
        if let Some(current_scene) = self.scenes.last_mut() {
            match current_scene.update(ctx, window) {
                SceneSwitch::None => {}
                SceneSwitch::Push(new_scene) => self.scenes.push(new_scene),
                SceneSwitch::Pop => {
                    self.scenes.pop();
                }
                SceneSwitch::Replace(new_scene) => {
                    self.scenes.pop();
                    self.scenes.push(new_scene);
                }
                SceneSwitch::Quit => return false,
            }
        }
        true
    }

    /// Renders the current scene.
    pub fn render(&mut self, gl: &Arc<glow::Context>, ui: &mut UIRenderer) {
        if let Some(current_scene) = self.scenes.last_mut() {
            current_scene.render(gl, ui);
        }
    }
}

pub mod singleplayer;
pub mod titlescreen;
