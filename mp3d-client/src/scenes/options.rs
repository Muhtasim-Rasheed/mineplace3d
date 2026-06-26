use std::sync::{Arc, RwLock};

use glam::Vec2;
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::{Assets, SceneAction, SceneUpdateContext},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub username: String,
    pub fullscreen: Option<bool>,
    pub sensitivity: Option<f32>,
    pub resource_packs: Option<Vec<String>>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            username: "Player".to_string(),
            fullscreen: Some(false),
            sensitivity: Some(1.0),
            resource_packs: Some(vec![]),
        }
    }
}

impl ClientConfig {
    pub fn load() -> Self {
        let config_path = crate::get_config_path();
        if config_path.exists() {
            let config_data = std::fs::read_to_string(config_path).unwrap();
            serde_json::from_str(&config_data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let config_path = crate::get_config_path();
        let config_data = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(config_path, config_data).unwrap();
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen.unwrap_or(false)
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity.unwrap_or(1.0)
    }

    pub fn resource_packs(&self) -> &[String] {
        self.resource_packs.as_deref().unwrap_or(&[])
    }
}

pub struct Options {
    container: Column,
}

impl Options {
    pub fn new(
        assets: &Arc<Assets>,
        window_size: (u32, u32),
        config: &Arc<RwLock<ClientConfig>>,
    ) -> Self {
        let mut container = Column::new(40.0)
            .justification(Justification::Center)
            .with(Label::new("Options").font_size(48.0))
            .with(
                Column::new(20.0)
                    .with(
                        InputField::new("Username")
                            .sanitize("/\\?%*:|\"<> ")
                            .text(&config.read().unwrap().username),
                    )
                    .with(Button::new(&format!(
                        "Fullscreen: {}",
                        if config.read().unwrap().fullscreen() {
                            "On"
                        } else {
                            "Off"
                        }
                    )))
                    .with(Button::new("Clear Logs"))
                    .with(
                        Slider::new("Mouse Sensitivity", Vec2::new(500.0, 80.0), 0.1..=2.0)
                            .value(config.read().unwrap().sensitivity()),
                    )
                    .with(Button::new("Resource Packs"))
                    .with(Button::new("Back")),
            );

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        Self { container }
    }
}

impl super::Scene for Options {
    fn update(&mut self, ctx: &mut SceneUpdateContext) -> Vec<SceneAction> {
        let SceneUpdateContext {
            ctx,
            window,
            sdl_ctx,
            assets,
            config,
            ..
        } = ctx;

        window.set_title("Mineplace3D - Options").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);

        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        self.container
            .find_widget_mut::<Button>(&[1, 1])
            .unwrap()
            .text = format!(
            "Fullscreen: {}",
            if config.read().unwrap().fullscreen() {
                "On"
            } else {
                "Off"
            }
        );

        if self
            .container
            .find_widget::<Button>(&[1, 1])
            .unwrap()
            .is_released()
        {
            let mut config_guard = config.write().unwrap();
            config_guard.fullscreen = Some(!config_guard.fullscreen());
            config_guard.save();

            log::info!("Toggled fullscreen: {}", config_guard.fullscreen());

            window
                .set_fullscreen(if config_guard.fullscreen() {
                    sdl2::video::FullscreenType::Desktop
                } else {
                    sdl2::video::FullscreenType::Off
                })
                .unwrap();
        }

        let input_text = self
            .container
            .find_widget::<InputField>(&[1, 0])
            .unwrap()
            .text
            .clone();

        self.container
            .find_widget_mut::<Button>(&[1, 5])
            .unwrap()
            .disabled = input_text.trim().is_empty();

        if self
            .container
            .find_widget::<Button>(&[1, 2])
            .unwrap()
            .is_released()
        {
            log::info!("Clearing logs...");

            let game_dir = crate::get_game_dir();
            if let Ok(entries) = std::fs::read_dir(game_dir) {
                let mut log_count = 0;
                for entry in entries.flatten() {
                    let path = entry.path();
                    // Do not remove the current log file.
                    if path.extension().and_then(|s| s.to_str()) == Some("log")
                        && path.file_name().and_then(|s| s.to_str()) != Some("game.log")
                    {
                        log_count += 1;
                        let _ = std::fs::remove_file(path);
                    }
                }

                log::info!("Cleared {} log(s)", log_count);
            }
        }

        if self
            .container
            .find_widget::<Button>(&[1, 5])
            .unwrap()
            .is_released()
        {
            let mut config_guard = config.write().unwrap();
            config_guard.username = input_text;
            config_guard.sensitivity =
                Some(self.container.find_widget::<Slider>(&[1, 3]).unwrap().value);
            config_guard.save();

            log::info!("Saved config: {:?}", *config_guard);

            return vec![SceneAction::Pop];
        }

        if self
            .container
            .find_widget::<Button>(&[1, 4])
            .unwrap()
            .is_released()
        {
            return vec![SceneAction::Push(Box::new(
                super::packselection::PackSelection::new(
                    config.read().unwrap().resource_packs(),
                    assets,
                    window.size(),
                ),
            ))];
        }

        Vec::new()
    }

    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut UIRenderer,
        assets: &Arc<Assets>,
        _config: &Arc<RwLock<super::options::ClientConfig>>,
    ) {
        unsafe {
            gl.clear_color(0.1, 0.1, 0.2, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.container.draw(ui, assets);
        }
    }
}
