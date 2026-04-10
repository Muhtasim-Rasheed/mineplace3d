use std::sync::{Arc, RwLock};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::Assets,
};

pub struct WorldSelection {
    container: Row,
    selected: Option<usize>,
    previous_worlds: Vec<String>,
}

impl WorldSelection {
    pub fn new(assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let header = Label::new("Select World", 48.0, Vec4::ONE);

        let create_button =
            Button::new("Create New World", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));

        let mut join_button = Button::new("Join World", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));

        let mut delete_button = Button::new(
            "Delete World",
            Vec4::new(1.0, 0.5, 0.5, 1.0),
            24.0,
            Vec2::new(500.0, 80.0),
        );

        join_button.disabled = true;
        delete_button.disabled = true;

        let back_button = Button::new("Back", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));

        let mut buttons = Column::new(
            5.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            None,
        );
        buttons.add_widget(create_button);
        buttons.add_widget(join_button);
        buttons.add_widget(delete_button);
        buttons.add_widget(back_button);

        let mut panel = Column::new(
            30.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            None,
        );
        panel.add_widget(header);
        panel.add_widget(buttons);

        let mut world_list = Column::new(
            5.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            None,
        );
        for world in Self::get_worlds() {
            let world_button = Button::new(&world, Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
            world_list.add_widget(world_button);
        }

        let mut column = Column::new(
            40.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            Some(window_size.1 as f32 - 200.0),
        );
        column.add_widget(world_list);

        let mut container = Row::new(
            20.0,
            Alignment::Start,
            Vec4::new(0.0, 0.0, 60.0, 40.0),
            Justification::Center,
        );
        container.add_widget(panel);
        container.add_widget(column);

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
    fn update(
        &mut self,
        gl: &Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        window: &mut sdl2::video::Window,
        _sdl_ctx: &sdl2::Sdl,
        assets: &Arc<Assets>,
        config: &Arc<RwLock<super::ClientConfig>>,
    ) -> super::SceneAction {
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
            return super::SceneAction::Pop;
        }

        if self.previous_worlds != Self::get_worlds() {
            self.previous_worlds = Self::get_worlds();
            let world_list = self.container.find_widget_mut::<Column>(&[1, 0]).unwrap();
            world_list.widgets.clear();
            for world in &self.previous_worlds {
                let world_button = Button::new(world, Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
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
            return super::SceneAction::Push(Box::new(super::worldcreation::WorldCreation::new(
                assets,
                window.size(),
            )));
        }

        if self
            .container
            .find_widget::<Button>(&[0, 1, 1])
            .unwrap()
            .is_released()
        {
            let world_name = self.previous_worlds[self.selected.unwrap()].clone();
            log::info!("Joining world {}", world_name);
            return super::SceneAction::Push(Box::new(super::singleplayer::SinglePlayer::load(
                gl,
                assets,
                window.size(),
                crate::get_saves_dir().join(world_name),
                config.read().unwrap().username.clone(),
            )));
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
            return super::SceneAction::Pop;
        }

        super::SceneAction::None
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
