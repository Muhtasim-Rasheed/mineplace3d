use std::sync::{Arc, RwLock};

use glam::Vec2;
use glow::HasContext;

use crate::{
    render::ui::{
        uirenderer::UIRenderer,
        widgets::{Button, Column, InputField, Justification, Label, LayoutContext, Row, Widget},
    },
    scenes::{Assets, SceneAction, SceneUpdateContext},
};

pub struct JoinServer {
    container: Column,
}

impl JoinServer {
    pub fn new(assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let mut container = Column::new(40.0)
            .justification(Justification::Center)
            .with(Label::new("Join Server").font_size(48.0))
            .with(
                Column::new(10.0)
                    .with(InputField::new("Server Address"))
                    .with(InputField::new("Password"))
                    .with(
                        Row::new(10.0)
                            .with(Button::new("Back"))
                            .with(Button::new("Join")),
                    ),
            );

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        Self { container }
    }
}

impl super::Scene for JoinServer {
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

        window.set_title("Mineplace3D - Join Server").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);

        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        if self
            .container
            .find_widget::<Button>(&[1, 2, 0])
            .unwrap()
            .is_released()
        {
            return vec![SceneAction::Pop];
        }

        if self
            .container
            .find_widget::<Button>(&[1, 2, 1])
            .unwrap()
            .is_released()
        {
            let multiplayer_instance = super::gamescene::GameScene::multiplayer(
                gl,
                assets,
                window.size(),
                &self
                    .container
                    .find_widget::<InputField>(&[1, 0])
                    .unwrap()
                    .text,
                config.read().unwrap().username.clone(),
                self.container
                    .find_widget::<InputField>(&[1, 1])
                    .unwrap()
                    .text
                    .clone(),
            );
            match multiplayer_instance {
                Ok(i) => {
                    log::info!("Connecting to server");
                    return vec![SceneAction::Push(Box::new(i))];
                }
                Err(e) => {
                    log::error!("Failed to load world: {}", e);
                    return vec![SceneAction::ShowError(
                        crate::scenes::SceneActionError::FailedConnectingServer(format!("{}", e)),
                    )];
                }
            }
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
