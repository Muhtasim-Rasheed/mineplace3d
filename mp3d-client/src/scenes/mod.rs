//! Module providing the `Scene` trait and all scene implementations.
//!
//! This module serves as a central point for managing different scenes in the game client.

use std::{collections::HashMap, sync::Arc};

use crate::render::ui::uirenderer::UIRenderer;

pub enum SceneSwitch {
    None,
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
    Quit,
}

/// Assets given to scenes during update and render, which they can use to access resources such
/// as block textures and models.
pub struct Assets {
    pub block_textures: crate::resource::block::TextureAtlas,
    pub block_models: HashMap<&'static str, crate::resource::block::BlockModel>,
}

impl Assets {
    /// Loads all assets needed for the scenes.
    ///
    /// A blank `TextureAtlas` is created and passed to each `BlockModel` as it is loaded, allowing
    /// them to add their textures to the atlas as they are loaded. This ensures that only the
    /// needed textures are loaded into the atlas.
    pub fn load(gl: &Arc<glow::Context>) -> Result<Self, String> {
        let mut block_textures = crate::resource::block::TextureAtlas::new(256, 16);
        let mut block_models = HashMap::new();
        for block in mp3d_core::block::Block::ALL_BLOCKS {
            let model = crate::resource::block::BlockModel::from_block(&block, &mut block_textures)
                .map_err(|e| format!("Failed to load model for block '{}': {}", block.ident, e))?;
            block_models.insert(block.ident, model);
        }
        block_textures.upload(gl);
        block_textures.free_cpu_memory();
        Ok(Self {
            block_textures,
            block_models,
        })
    }
}

/// The Scene trait defines the common interface for all scenes in the game client.
pub trait Scene {
    /// Handles an event.
    fn handle_event(&mut self, _gl: &std::sync::Arc<glow::Context>, _event: &sdl2::event::Event) {}

    /// Updates the scene state.
    fn update(
        &mut self,
        _gl: &Arc<glow::Context>,
        _ctx: &crate::other::UpdateContext,
        _window: &mut sdl2::video::Window,
        _sdl_ctx: &sdl2::Sdl,
        _assets: &Arc<Assets>,
    ) -> SceneSwitch {
        SceneSwitch::None
    }

    /// Renders the scene.
    fn render(&mut self, gl: &Arc<glow::Context>, ui: &mut UIRenderer, assets: &Arc<Assets>);
}

/// Manages the stack of scenes.
pub struct SceneManager {
    assets: Arc<Assets>,
    scenes: Vec<Box<dyn Scene>>,
    just_switched: bool,
}

impl SceneManager {
    /// Creates a new SceneManager with the initial scene.
    pub fn new(initial_scene: Box<dyn Scene>, assets: Assets) -> Self {
        Self {
            assets: Arc::new(assets),
            scenes: vec![initial_scene],
            just_switched: false,
        }
    }

    /// Handles an event by passing it to the current scene.
    pub fn handle_event(&mut self, gl: &std::sync::Arc<glow::Context>, event: &sdl2::event::Event) {
        if let Some(current_scene) = self.scenes.last_mut() {
            current_scene.handle_event(gl, event);
        }
    }

    /// Updates the current scene and manages scene transitions.
    pub fn update(
        &mut self,
        gl: &Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        window: &mut sdl2::video::Window,
        sdl_ctx: &sdl2::Sdl,
    ) -> bool {
        if self.just_switched {
            self.just_switched = false;
            return true;
        }
        if let Some(current_scene) = self.scenes.last_mut() {
            let switch = current_scene.update(gl, ctx, window, sdl_ctx, &self.assets);
            let is_switching = !matches!(switch, SceneSwitch::None);
            match switch {
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
            if is_switching {
                self.just_switched = true;
            }
        }
        true
    }

    /// Renders the current scene.
    pub fn render(&mut self, gl: &Arc<glow::Context>, ui: &mut UIRenderer) {
        if let Some(current_scene) = self.scenes.last_mut() {
            current_scene.render(gl, ui, &self.assets);
        }
    }
}

pub mod singleplayer;
pub mod titlescreen;
