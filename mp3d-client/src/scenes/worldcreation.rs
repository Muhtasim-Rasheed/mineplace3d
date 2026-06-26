use std::sync::{Arc, RwLock};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::{Assets, SceneAction, SceneUpdateContext},
};

pub struct WorldCreation {
    container: Column,
    world_path: std::path::PathBuf,
}

impl WorldCreation {
    pub fn new(assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let world_path = crate::get_saves_dir().join("New_World");

        let mut container = Column::new(20.0)
            .with(Label::new("Create New World").font_size(48.0))
            .with(
                Column::new(20.0)
                    .with(
                        InputField::new("World Name")
                            .sanitize("/\\?%*:|\"<> ")
                            .text("New_World"),
                    )
                    .with(
                        Label::new(&world_path.display().to_string())
                            .color(Vec4::new(0.8, 0.8, 0.8, 1.0)),
                    )
                    .with(InputField::new("Seed (optional)")),
            )
            .with(
                Row::new(60.0)
                    .with(Button::new("Cancel"))
                    .with(Button::new("Create")),
            );

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        Self {
            container,
            world_path,
        }
    }
}

impl super::Scene for WorldCreation {
    fn update(&mut self, ctx: &mut SceneUpdateContext) -> Vec<SceneAction> {
        let SceneUpdateContext {
            gl,
            ctx,
            window,
            sdl_ctx,
            assets,
            config,
            ..
        } = ctx;

        window.set_title("Mineplace3D - Create world").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);

        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        if ctx
            .keyboard
            .pressed
            .contains(&sdl2::keyboard::Keycode::Escape)
        {
            return vec![SceneAction::Pop];
        }

        self.world_path = crate::get_saves_dir().join(
            self.container
                .find_widget::<InputField>(&[1, 0])
                .and_then(|input| {
                    let text = input.text.trim();
                    if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    }
                })
                .unwrap_or_else(|| "New_World".to_string()),
        );

        if let Some(label) = self.container.find_widget_mut::<Label>(&[1, 1]) {
            label.text = self.world_path.display().to_string();
        }

        if let Some(create_button) = self.container.find_widget_mut::<Button>(&[2, 1]) {
            create_button.disabled = self.world_path.exists();
        }

        if let Some(cancel_button) = self.container.find_widget::<Button>(&[2, 0])
            && cancel_button.is_pressed()
        {
            return vec![SceneAction::Pop];
        }

        let seed =
            self.container
                .find_widget::<InputField>(&[1, 2])
                .map_or(rand::random(), |input| {
                    let text = input.text.trim();
                    if let Ok(num) = text.parse::<i32>() {
                        num
                    } else if !text.trim().is_empty() {
                        fxhash::hash32(text.as_bytes()) as i32
                    } else {
                        rand::random()
                    }
                });

        if let Some(create_button) = self.container.find_widget::<Button>(&[2, 1])
            && create_button.is_pressed()
        {
            log::info!(
                "New world at {} with seed {}",
                self.world_path.display(),
                seed
            );
            return vec![SceneAction::Replace(Box::new(
                super::singleplayer::SinglePlayer::new(
                    gl,
                    assets,
                    window.size(),
                    seed,
                    self.world_path.clone(),
                    config.read().unwrap().username.clone(),
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
