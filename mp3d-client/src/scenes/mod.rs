//! Module providing the `Scene` trait and all scene implementations.
//!
//! This module serves as a central point for managing different scenes in the game client.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use glow::HasContext;
use image::GenericImageView;
use mp3d_core::block::{BlockId, BlockState, block_registry};

use crate::{
    render::{
        dialog::draw_dialog,
        ui::{uirenderer::UIRenderer, widgets::Font},
    },
    resource::{
        ResourceManager,
        block::{BlockModel, States, TextureAtlas},
    },
    scenes::options::ClientConfig,
};

pub enum SceneAction {
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
    Quit,
    ReloadAssets,
    ShowError(SceneActionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneActionError {
    Debug,
    FailedReloadingAssets(String),
    FailedLoadingWorld(String),
    Unexpected(String),
}

impl std::fmt::Display for SceneActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneActionError::Debug => write!(
                f,
                "Error triggered via F12\n\nLorem molestias ad repellendus iste ut. Laborum sit magni quaerat maxime eum animi Voluptatem animi illum quibusdam nihil consequatur nisi accusamus rem? Vitae repudiandae deserunt molestiae doloremque qui. Voluptates laudantium."
            ),
            SceneActionError::FailedReloadingAssets(e) => {
                write!(f, "Failed reloading assets\n\n{}", e)
            }
            SceneActionError::FailedLoadingWorld(e) => {
                write!(
                    f,
                    "Failed loading '{}'\n\nMore details are likely available in the log file.",
                    e
                )
            }
            SceneActionError::Unexpected(e) => {
                write!(f, "An unexpected error occurred, but is not fatal\n\n{}", e)
            }
        }
    }
}

impl std::error::Error for SceneActionError {}

pub type SceneActionResult = Result<(), SceneActionError>;

/// Assets given to scenes during update and render, which they can use to access resources such
/// as block textures and models.
pub struct Assets {
    pub block_textures: TextureAtlas,
    pub block_models: HashMap<(BlockId, u16), BlockModel>,
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
        for (block_id, block) in block_registry().iter_enumerate() {
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
                    state.transform,
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
                block_models.insert((block_id, state_data), model);
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
            block_registry().len()
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

#[allow(unused)]
pub struct SceneUpdateContext<'a> {
    pub gl: &'a Arc<glow::Context>,
    pub ctx: &'a crate::other::UpdateContext<'a>,
    pub window: &'a mut sdl2::video::Window,
    pub sdl_ctx: &'a sdl2::Sdl,
    pub assets: &'a Arc<Assets>,
    pub config: &'a Arc<RwLock<ClientConfig>>,
    pub result: &'a SceneActionResult,
}

/// The Scene trait defines the common interface for all scenes in the game client.
pub trait Scene {
    /// Handles an event.
    fn handle_event(&mut self, _gl: &std::sync::Arc<glow::Context>, _event: &sdl2::event::Event) {}

    /// Updates the scene state.
    fn update(&mut self, _ctx: &mut SceneUpdateContext) -> Vec<SceneAction> {
        Vec::new()
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
    timer: f32,
    last_err_time: f32,
    last_err: Option<SceneActionError>,
    result: SceneActionResult,
}

impl SceneManager {
    /// Creates a new SceneManager with the initial scene.
    pub fn new(initial_scene: Box<dyn Scene>, assets: Arc<Assets>, config: ClientConfig) -> Self {
        Self {
            assets,
            config: Arc::new(RwLock::new(config)),
            scenes: vec![initial_scene],
            just_switched: false,
            timer: 0.0,
            last_err_time: 0.0,
            last_err: None,
            result: Ok(()),
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
        self.timer += ctx.delta_time;
        if self.just_switched {
            self.just_switched = false;
            return true;
        }
        if ctx.keyboard.pressed.contains(&sdl2::keyboard::Keycode::F12) {
            self.result = Err(SceneActionError::Debug);
            self.last_err_time = self.timer;
            self.last_err = Some(SceneActionError::Debug);
            return true;
        }
        if let Some(current_scene) = self.scenes.last_mut() {
            let actions = current_scene.update(&mut SceneUpdateContext {
                gl,
                ctx,
                window,
                sdl_ctx,
                assets: &self.assets,
                config: &self.config,
                result: &self.result,
            });
            self.result = Ok(());
            let mut result_override = None;
            for action in actions {
                let does_switch = !matches!(
                    action,
                    SceneAction::ReloadAssets | SceneAction::ShowError(_)
                );
                match action {
                    SceneAction::Push(new_scene) => {
                        self.scenes.push(new_scene);
                        self.result = Ok(());
                    }
                    SceneAction::Pop => {
                        self.scenes.pop();
                        self.result = Ok(());
                    }
                    SceneAction::Replace(new_scene) => {
                        self.scenes.pop();
                        self.scenes.push(new_scene);
                        self.result = Ok(());
                    }
                    SceneAction::Quit => {
                        self.result = Ok(());
                        return false;
                    }
                    SceneAction::ReloadAssets => {
                        log::info!("Reloading assets...");
                        match Assets::load(gl, window, &self.config.read().unwrap()) {
                            Ok(new_assets) => {
                                self.assets = Arc::new(new_assets);
                                log::info!("Assets reloaded successfully");
                                self.result = Ok(());
                            }
                            Err(e) => {
                                log::error!("Failed to reload assets: {}", e);
                                self.result = Err(SceneActionError::FailedReloadingAssets(e));
                            }
                        }
                    }
                    SceneAction::ShowError(e) => {
                        result_override = Some(Err(e));
                    }
                }
                if does_switch {
                    self.just_switched = true;
                }
            }

            if let Some(result) = result_override {
                self.result = result;
            }
        }
        if let Err(e) = &self.result {
            self.last_err_time = self.timer;
            self.last_err = Some(e.clone());
        }
        true
    }

    /// Renders the current scene.
    pub fn render(&mut self, gl: &Arc<glow::Context>, ui: &mut UIRenderer) {
        if let Some(current_scene) = self.scenes.last_mut() {
            current_scene.render(gl, ui, &self.assets, &self.config);
        }

        unsafe {
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::DEPTH_TEST);
        }

        if let Some(err) = &self.last_err {
            draw_dialog(
                &format!("{}", err),
                &self.assets,
                ui,
                self.timer,
                self.last_err_time,
            );
        }
    }
}

pub mod options;
pub mod packselection;
pub mod singleplayer;
pub mod titlescreen;
pub mod worldcreation;
pub mod worldselection;
