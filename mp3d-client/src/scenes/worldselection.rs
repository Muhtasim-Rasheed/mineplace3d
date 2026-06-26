use std::sync::{Arc, RwLock};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::{Assets, SceneAction, SceneUpdateContext},
};

pub struct WorldSelection {
    container: Row,
    selected: Option<usize>,
    previous_worlds: Vec<String>,
}

impl WorldSelection {
    pub fn new(assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let mut container = Row::new(20.0)
            .alignment(Alignment::Start)
            .justification(Justification::Center)
            .padding(Vec4::new(0.0, 0.0, 60.0, 40.0))
            .with(
                Column::new(30.0)
                    .with(Label::new("Select World").font_size(48.0))
                    .with(
                        Column::new(5.0)
                            .with(Button::new("Create New World"))
                            .with(Button::new("Join World").disabled())
                            .with(
                                Button::new("Delete World")
                                    .color(Vec4::new(1.0, 0.5, 0.5, 1.0))
                                    .disabled(),
                            )
                            .with(Button::new("Back")),
                    ),
            )
            .with(
                Column::new(40.0)
                    .viewport_height(window_size.1 as f32 - 200.0)
                    .with(
                        Column::new(5.0)
                            .with_many(Self::get_worlds().into_iter().map(|v| Button::new(&v))),
                    ),
            );

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        Self {
            container,
            selected: None,
            previous_worlds: Self::get_worlds(),
        }
    }

    fn get_worlds() -> Vec<String> {
        let saves_dir = crate::get_saves_dir();
        if let Ok(entries) = std::fs::read_dir(saves_dir) {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl super::Scene for WorldSelection {
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

        window.set_title("Mineplace3D - Select world").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);

        self.container
            .get_widget_mut::<Column>(1)
            .unwrap()
            .viewport_height = Some(window.size().1 as f32 - 200.0);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });
        self.container.update(ctx);

        if ctx
            .keyboard
            .pressed
            .contains(&sdl2::keyboard::Keycode::Escape)
        {
            return vec![SceneAction::Pop];
        }

        if self.previous_worlds != Self::get_worlds() {
            self.previous_worlds = Self::get_worlds();
            let world_list = self.container.find_widget_mut::<Column>(&[1, 0]).unwrap();
            world_list.widgets.clear();
            for world in &self.previous_worlds {
                let world_button = Button::new(world);
                world_list.add_widget(world_button);
            }
        }

        let mut newly_selected = None;
        let len = self
            .container
            .find_widget::<Column>(&[1, 0])
            .unwrap()
            .widgets
            .len();
        for i in 0..len {
            let button = self
                .container
                .find_widget_mut::<Button>(&[1, 0, i])
                .unwrap();
            if button.is_released() {
                newly_selected = Some(i);
            }
        }

        if let Some(new_selection) = newly_selected {
            self.selected = Some(new_selection);
        }

        for i in 0..len {
            let button = self
                .container
                .find_widget_mut::<Button>(&[1, 0, i])
                .unwrap();
            button.always_hovered = Some(i) == self.selected;
        }

        self.container
            .find_widget_mut::<Button>(&[0, 1, 1])
            .unwrap()
            .disabled = self.selected.is_none();
        self.container
            .find_widget_mut::<Button>(&[0, 1, 2])
            .unwrap()
            .disabled = self.selected.is_none();

        if self
            .container
            .find_widget::<Button>(&[0, 1, 0])
            .unwrap()
            .is_released()
        {
            log::info!("Creating new world");
            return vec![SceneAction::Push(Box::new(
                super::worldcreation::WorldCreation::new(assets, window.size()),
            ))];
        }

        if self
            .container
            .find_widget::<Button>(&[0, 1, 1])
            .unwrap()
            .is_released()
        {
            let world_name = self.previous_worlds[self.selected.unwrap()].clone();
            let singleplayer_instance = super::singleplayer::SinglePlayer::load(
                gl,
                assets,
                window.size(),
                crate::get_saves_dir().join(world_name.clone()),
                config.read().unwrap().username.clone(),
            );
            if let Ok(singleplayer_instance) = singleplayer_instance {
                log::info!("Joining world {}", world_name);
                return vec![SceneAction::Push(Box::new(singleplayer_instance))];
            } else {
                log::error!(
                    "Failed to load world: {}",
                    singleplayer_instance.err().unwrap()
                );
                return vec![SceneAction::ShowError(
                    crate::scenes::SceneActionError::FailedLoadingWorld(world_name),
                )];
            }
        }

        if self
            .container
            .find_widget::<Button>(&[0, 1, 2])
            .unwrap()
            .is_released()
        {
            let world_name = self.previous_worlds[self.selected.unwrap()].clone();
            let world_path = crate::get_saves_dir().join(world_name);
            if std::fs::remove_dir_all(&world_path).is_ok() {
                self.selected = None;
            }
            self.container.layout(&LayoutContext {
                max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
                cursor: Vec2::ZERO,
                assets,
            });
        }

        if self
            .container
            .find_widget::<Button>(&[0, 1, 3])
            .unwrap()
            .is_released()
        {
            return vec![SceneAction::Pop];
        }

        Vec::new()
    }

    fn render(
        &mut self,
        gl: &Arc<glow::Context>,
        ui: &mut UIRenderer,
        assets: &Arc<Assets>,
        _config: &Arc<RwLock<super::ClientConfig>>,
    ) {
        unsafe {
            gl.clear_color(0.1, 0.1, 0.2, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.container.draw(ui, assets);
        }
    }
}
