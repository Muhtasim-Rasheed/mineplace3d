//! Module providing the `Scene` trait and all scene implementations.
//!
//! This module serves as a central point for managing different scenes in the game client.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use image::GenericImageView;
use mp3d_core::block::BlockState;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::Font},
    resource::{
        ResourceManager,
        block::{BlockModel, States, TextureAtlas},
    },
    scenes::options::ClientConfig,
};

pub enum SceneAction {
    None,
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
    Quit,
    ReloadAssets,
    ReloadAssetsAndPop,
}

/// Assets given to scenes during update and render, which they can use to access resources such
/// as block textures and models.
pub struct Assets {
    pub block_textures: TextureAtlas,
    pub block_models: HashMap<(&'static str, u16), BlockModel>,
    pub font: Font,
    pub gui_tex: crate::abs::Texture,
}

impl Assets {
    /// Loads all assets needed for the scenes.
    ///
    /// A blank `TextureAtlas` is created and passed to each `BlockModel` as it is loaded, allowing
    /// them to add their textures to the atlas as they are loaded. This ensures that only the
    /// needed textures are loaded into the atlas.
    pub fn load(
        gl: &Arc<glow::Context>,
        window: &mut sdl2::video::Window,
        config: &ClientConfig,
    ) -> Result<Self, String> {
        let resource_manager = ResourceManager::new(config.resource_packs());
        let mut block_textures = TextureAtlas::new(256, 16);
        let mut block_models = HashMap::new();
        for block in mp3d_core::block::Block::ALL_BLOCKS {
            let mut possible_state_data_values = BlockState::possible_data_values(block.state_type)
                .unwrap()
                .iter()
                .collect::<std::collections::HashSet<_>>();
            let blockstate_path = PathBuf::from(format!("blocks/states/{}.json", block.ident));
            let blockstate_data = resource_manager
                .read(&blockstate_path)
                .ok_or_else(|| format!("Failed to load blockstate for block '{}'", block.ident))?;
            let blockstate_str = std::str::from_utf8(&blockstate_data).map_err(|e| {
                format!(
                    "Failed to parse blockstate for block '{}': {}",
                    block.ident, e
                )
            })?;
            let states = States::load(blockstate_str).map_err(|e| {
                format!(
                    "Failed to parse blockstate for block '{}': {}",
                    block.ident, e
                )
            })?;
            for (state_data, state) in states.states {
                let model_path = state.model;
                let model_file = resource_manager.read(&model_path).ok_or_else(|| {
                    format!(
                        "Failed to load model file '{}' for block '{}'",
                        model_path.display(),
                        block.ident
                    )
                })?;
                let model_file = std::str::from_utf8(&model_file).map_err(|e| {
                    format!(
                        "Failed to parse model file '{}' for block '{}': {}",
                        model_path.display(),
                        block.ident,
                        e
                    )
                })?;
                let model = BlockModel::from_block(
                    model_path,
                    model_file,
                    &resource_manager,
                    &mut block_textures,
                )?;
                if !possible_state_data_values.contains(&state_data) {
                    log::warn!(
                        "State data value {:#06x} for block '{}' is not valid for its state type",
                        state_data,
                        block.ident
                    );
                }
                possible_state_data_values.remove(&state_data);
                block_models.insert((block.ident, state_data), model);
            }

            if !possible_state_data_values.is_empty() {
                log::error!(
                    "Not all possible state data values for block '{}' were used in the blockstate file. Unused values: {:?}",
                    block.ident,
                    possible_state_data_values
                );
                panic!("Invalid blockstate file for block '{}'", block.ident);
            }
        }
        block_textures.upload(gl);
        block_textures.free_cpu_memory();
        log::info!(
            "Loaded {} block textures and {} block models for {} blocks",
            block_textures.texture_count(),
            block_models.len(),
            mp3d_core::block::Block::ALL_BLOCKS.len()
        );
        let font = Font::new(
            crate::abs::Texture::new(
                gl,
                &image::load_from_memory_with_format(
                    &resource_manager
                        .read(std::path::Path::new("font.png"))
                        .ok_or_else(|| "Failed to load font texture".to_string())?,
                    image::ImageFormat::Png,
                )
                .unwrap(),
            ),
            resource_manager
                .read(std::path::Path::new("font.json"))
                .ok_or_else(|| "Failed to load font metadata".to_string())
                .and_then(|data| {
                    serde_json::from_slice(&data)
                        .map_err(|e| format!("Failed to parse font metadata: {}", e))
                })?,
        );
        let gui_tex = crate::abs::Texture::new(
            gl,
            &image::load_from_memory_with_format(
                &resource_manager
                    .read(std::path::Path::new("gui.png"))
                    .ok_or_else(|| "Failed to load GUI texture".to_string())?,
                image::ImageFormat::Png,
            )
            .unwrap(),
        );
        let window_icon = image::load_from_memory_with_format(
            &resource_manager
                .read(std::path::Path::new("window_icon.png"))
                .ok_or_else(|| "Failed to load window icon".to_string())?,
            image::ImageFormat::Png,
        )
        .unwrap();
        let (icon_width, icon_height) = window_icon.dimensions();
        let mut icon_rgba = window_icon.into_rgba8().into_raw();
        let icon = sdl2::surface::Surface::from_data(
            &mut icon_rgba,
            icon_width,
            icon_height,
            icon_width * 4,
            sdl2::pixels::PixelFormatEnum::RGBA32,
        )
        .map_err(|e| format!("Failed to create window icon surface: {}", e))?;
        window.set_icon(icon);
        Ok(Self {
            block_textures,
            block_models,
            font,
            gui_tex,
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
        _config: &Arc<RwLock<ClientConfig>>,
    ) -> SceneAction {
        SceneAction::None
    }

    /// Renders the scene.
    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut UIRenderer,
        assets: &Arc<Assets>,
        config: &Arc<RwLock<ClientConfig>>,
    );
}

/// Manages the stack of scenes.
pub struct SceneManager {
    assets: Arc<Assets>,
    config: Arc<RwLock<ClientConfig>>,
    scenes: Vec<Box<dyn Scene>>,
    just_switched: bool,
}

impl SceneManager {
    /// Creates a new SceneManager with the initial scene.
    pub fn new(initial_scene: Box<dyn Scene>, assets: Arc<Assets>, config: ClientConfig) -> Self {
        Self {
            assets,
            config: Arc::new(RwLock::new(config)),
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
            let switch = current_scene.update(gl, ctx, window, sdl_ctx, &self.assets, &self.config);
            let is_switching = !matches!(switch, SceneAction::None | SceneAction::ReloadAssets);
            match switch {
                SceneAction::None => {}
                SceneAction::Push(new_scene) => self.scenes.push(new_scene),
                SceneAction::Pop => {
                    self.scenes.pop();
                }
                SceneAction::Replace(new_scene) => {
                    self.scenes.pop();
                    self.scenes.push(new_scene);
                }
                SceneAction::Quit => return false,
                SceneAction::ReloadAssets => {
                    log::info!("Reloading assets...");
                    match Assets::load(gl, window, &self.config.read().unwrap()) {
                        Ok(new_assets) => {
                            self.assets = Arc::new(new_assets);
                            log::info!("Assets reloaded successfully");
                        }
                        Err(e) => log::error!("Failed to reload assets: {}", e),
                    }
                }
                SceneAction::ReloadAssetsAndPop => {
                    log::info!("Reloading assets...");
                    match Assets::load(gl, window, &self.config.read().unwrap()) {
                        Ok(new_assets) => {
                            self.assets = Arc::new(new_assets);
                            log::info!("Assets reloaded successfully");
                        }
                        Err(e) => log::error!("Failed to reload assets: {}", e),
                    }
                    self.scenes.pop();
                }
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
            current_scene.render(gl, ui, &self.assets, &self.config);
        }
    }
}

pub mod options;
pub mod packselection;
pub mod singleplayer;
pub mod titlescreen;
pub mod worldcreation;
pub mod worldselection;
