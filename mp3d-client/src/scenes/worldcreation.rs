use std::{rc::Rc, sync::Arc};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    abs::TextureHandle,
    render::ui::{uirenderer::UIRenderer, widgets::*},
};

pub struct WorldCreation {
    container: Column,
    world_path: std::path::PathBuf,
    font: Rc<Font>,
    texture: TextureHandle,
}

impl WorldCreation {
    pub fn new(font: &Rc<Font>, gui_tex: TextureHandle, window_size: (u32, u32)) -> Self {
        let world_path = crate::get_saves_dir().join("New_World");

        let header = Label::new("Create New World", 48.0, Vec4::ONE, font);

        let mut name_input = InputField::new(
            "World Name",
            Vec4::ONE,
            24.0,
            Vec2::new(1010.0, 80.0),
            Some("/\\?%*:|\"<> "),
            font,
            gui_tex,
        );
        name_input.text = "New_World".to_string();
        name_input.cursor_pos = name_input.text.len();

        let path_label = Label::new(
            &world_path.display().to_string(),
            24.0,
            Vec4::new(0.8, 0.8, 0.8, 1.0),
            font,
        );

        let mut world_options =
            Column::new(20.0, Alignment::Center, Vec4::ZERO, Justification::Start);
        world_options.add_widget(name_input);
        world_options.add_widget(path_label);

        let cancel_button = Button::new(
            "Cancel",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );

        let create_button = Button::new(
            "Create",
            Vec4::ONE,
            24.0,
            Vec2::new(500.0, 80.0),
            font,
            gui_tex,
        );

        let mut buttons = Row::new(60.0, Alignment::Center, Vec4::ZERO, Justification::Start);
        buttons.add_widget(cancel_button);
        buttons.add_widget(create_button);

        let mut container = Column::new(
            20.0,
            Alignment::Center,
            Vec4::new(0.0, 0.0, 40.0, 60.0),
            Justification::SpaceBetween,
        );
        container.add_widget(header);
        container.add_widget(world_options);
        container.add_widget(buttons);

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
        });

        Self {
            container,
            world_path,
            font: font.clone(),
            texture: gui_tex,
        }
    }
}

impl super::Scene for WorldCreation {
    fn handle_event(&mut self, _gl: &std::sync::Arc<glow::Context>, _event: &sdl2::event::Event) {}

    fn update(
        &mut self,
        gl: &Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        window: &mut sdl2::video::Window,
        _sdl_ctx: &sdl2::Sdl,
    ) -> super::SceneSwitch {
        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
        });

        if ctx
            .keyboard
            .pressed
            .contains(&sdl2::keyboard::Keycode::Escape)
        {
            return super::SceneSwitch::Pop;
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

        self.container
            .find_widget_mut::<Label>(&[1, 1])
            .map(|label| {
                label.text = self.world_path.display().to_string();
            });

        self.container
            .find_widget_mut::<Button>(&[2, 1])
            .map(|button| button.disabled = self.world_path.exists());

        if let Some(cancel_button) = self.container.find_widget::<Button>(&[2, 0]) {
            if cancel_button.is_pressed() {
                return super::SceneSwitch::Pop;
            }
        }

        if let Some(create_button) = self.container.find_widget::<Button>(&[2, 1]) {
            if create_button.is_pressed() {
                return super::SceneSwitch::Replace(Box::new(
                    super::singleplayer::SinglePlayer::new(
                        gl,
                        &self.font,
                        self.texture,
                        window.size(),
                        self.world_path.clone(),
                    ),
                ));
            }
        }

        super::SceneSwitch::None
    }

    fn render(&mut self, gl: &Arc<glow::Context>, ui: &mut UIRenderer) {
        unsafe {
            gl.clear_color(0.1, 0.1, 0.2, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.disable(glow::DEPTH_TEST);
            self.container.draw(ui);
            gl.enable(glow::DEPTH_TEST);
        }
    }
}
