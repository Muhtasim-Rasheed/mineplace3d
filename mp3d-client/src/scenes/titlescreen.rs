//! The title screen scene implementation.

use std::sync::{Arc, RwLock};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::{Assets, SceneAction, SceneUpdateContext},
};

static SPLASHES: std::sync::OnceLock<Vec<(&str, Vec4)>> = std::sync::OnceLock::new();

fn get_random_splash() -> (&'static str, Vec4) {
    let splashes = SPLASHES.get_or_init(|| {
        let file = include_str!("../assets/splashes.txt");
        file.lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    None
                } else {
                    let mut parts = line.rsplitn(2, '|');
                    let text = parts.next().unwrap().trim();
                    let color_code = parts.next().and_then(|s| s.trim().parse().ok());
                    let color = match color_code {
                        Some(code) => {
                            mp3d_core::textcomponent::TextComponentColor::Basic(code).into()
                        }
                        None => Vec4::new(rand::random(), rand::random(), rand::random(), 1.0),
                    };
                    Some((text, color))
                }
            })
            .collect()
    });
    let idx = rand::random::<u32>() as usize % splashes.len();
    splashes[idx]
}

/// The [`TitleScreen`] struct represents the title screen scene.
pub struct TitleScreen {
    container: Column,
}

impl TitleScreen {
    /// Creates a new [`TitleScreen`] instance.
    pub fn new(assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let (splash_text, splash_color) = get_random_splash();

        let (button_size, half_button_size) = if window_size.0 >= 1050 {
            (Vec2::new(1010.0, 80.0), Vec2::new(500.0, 80.0))
        } else {
            (
                Vec2::new(window_size.0 as f32 - 40.0, 80.0),
                Vec2::new((window_size.0 as f32 - 40.0 - 5.0) / 2.0, 80.0),
            )
        };

        let mut container = Column::new(50.0)
            .justification(Justification::SpaceBetween)
            .padding(Vec4::new(20.0, 20.0, 60.0, 20.0))
            .with(
                Column::new(5.0)
                    .with(Label::new("Mineplace3D").font_size(72.0))
                    .with(Label::new(splash_text).color(splash_color)),
            )
            .with(
                Column::new(10.0)
                    .with(Button::new("Singleplayer").size(button_size))
                    .with(
                        Row::new(10.0)
                            .with(Button::new("Options").size(half_button_size))
                            .with(Button::new("Quit").size(half_button_size)),
                    ),
            )
            .with(
                Row::new(5.0)
                    .justification(Justification::SpaceBetween)
                    .with(
                        Label::new(format!("Version {}", env!("CARGO_PKG_VERSION")).as_str())
                            .color(Vec4::new(1.0, 1.0, 1.0, 0.5)),
                    )
                    .with(Label::new("MIT License").color(Vec4::new(1.0, 1.0, 1.0, 0.5))),
            );

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        Self { container }
    }
}

impl super::Scene for TitleScreen {
    fn update(&mut self, ctx: &mut SceneUpdateContext) -> Vec<SceneAction> {
        let SceneUpdateContext {
            ctx,
            window,
            sdl_ctx,
            assets,
            config,
            ..
        } = ctx;

        window.set_title("Mineplace3D").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);

        let new_size = window.size();
        let container_padding_left_right = self.container.padding.x + self.container.padding.y;
        self.container.get_widget_mut::<Row>(2).unwrap().min_size =
            Vec2::new(new_size.0 as f32 - container_padding_left_right, 0.0);

        if new_size.0 >= 1050 {
            self.container
                .find_widget_mut::<Button>(&[1, 0])
                .unwrap()
                .size = Vec2::new(1010.0, 80.0);
            self.container
                .find_widget_mut::<Button>(&[1, 1, 0])
                .unwrap()
                .size = Vec2::new(500.0, 80.0);
            self.container
                .find_widget_mut::<Button>(&[1, 1, 1])
                .unwrap()
                .size = Vec2::new(500.0, 80.0);
        } else {
            self.container
                .find_widget_mut::<Button>(&[1, 0])
                .unwrap()
                .size = Vec2::new(new_size.0 as f32 - 40.0, 80.0);
            self.container
                .find_widget_mut::<Button>(&[1, 1, 0])
                .unwrap()
                .size = Vec2::new((new_size.0 as f32 - 40.0 - 5.0) / 2.0, 80.0);
            self.container
                .find_widget_mut::<Button>(&[1, 1, 1])
                .unwrap()
                .size = Vec2::new((new_size.0 as f32 - 40.0 - 5.0) / 2.0, 80.0);
        }

        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        if self
            .container
            .find_widget::<Button>(&[1, 0])
            .is_some_and(|btn| btn.is_released())
        {
            return vec![SceneAction::Push(Box::new(
                crate::scenes::worldselection::WorldSelection::new(assets, window.size()),
            ))];
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 0])
            .is_some_and(|btn| btn.is_released())
        {
            return vec![SceneAction::Push(Box::new(super::options::Options::new(
                assets,
                window.size(),
                config,
            )))];
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 1])
            .is_some_and(|btn| btn.is_released())
        {
            return vec![SceneAction::Quit];
        }

        Vec::new()
    }

    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut UIRenderer,
        assets: &Arc<super::Assets>,
        _config: &Arc<RwLock<super::options::ClientConfig>>,
    ) {
        unsafe {
            gl.clear_color(0.1, 0.1, 0.2, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.container.draw(ui, assets);
        }
    }
}
