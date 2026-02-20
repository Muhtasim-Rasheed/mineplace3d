//! The title screen scene implementation.

use std::{rc::Rc, sync::Arc};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    abs::TextureHandle,
    render::ui::{uirenderer::UIRenderer, widgets::*},
};

/// The [`TitleScreen`] struct represents the title screen scene.
pub struct TitleScreen {
    container: Column,
    font: Rc<Font>,
    texture: TextureHandle,
}

impl TitleScreen {
    /// Creates a new [`TitleScreen`] instance.
    pub fn new(font: &Rc<Font>, gui_tex: TextureHandle, window_size: (u32, u32)) -> Self {
        let header = Label::new("Mineplace3D", 72.0, Vec4::ONE, font);

        let play;
        let options;
        let quit;
        if window_size.0 >= 1050 {
            play = Button::new(
                "Start Game",
                Vec4::ONE,
                24.0,
                Vec2::new(1010.0, 80.0),
                font,
                gui_tex,
            );

            options = Button::new(
                "Options",
                Vec4::ONE,
                24.0,
                Vec2::new(500.0, 80.0),
                font,
                gui_tex,
            );

            quit = Button::new(
                "Quit",
                Vec4::ONE,
                24.0,
                Vec2::new(500.0, 80.0),
                font,
                gui_tex,
            );
        } else {
            play = Button::new(
                "Start Game",
                Vec4::ONE,
                24.0,
                Vec2::new(window_size.0 as f32 - 40.0, 80.0),
                font,
                gui_tex,
            );

            options = Button::new(
                "Options",
                Vec4::ONE,
                24.0,
                Vec2::new((window_size.0 as f32 - 40.0 - 5.0) / 2.0, 80.0),
                font,
                gui_tex,
            );

            quit = Button::new(
                "Quit",
                Vec4::ONE,
                24.0,
                Vec2::new((window_size.0 as f32 - 40.0 - 5.0) / 2.0, 80.0),
                font,
                gui_tex,
            );
        }

        let mut buttons_inner = Row::new(10.0, Alignment::Center, Vec4::ZERO, Justification::Start);
        buttons_inner.add_widget(options);
        buttons_inner.add_widget(quit);

        let mut buttons = Column::new(10.0, Alignment::Center, Vec4::ZERO, Justification::Start);
        buttons.add_widget(play);
        buttons.add_widget(buttons_inner);

        let version = Label::new(
            format!("Version {}", env!("CARGO_PKG_VERSION")).as_str(),
            24.0,
            Vec4::new(1.0, 1.0, 1.0, 0.5),
            font,
        );

        let license = Label::new("MIT License", 24.0, Vec4::new(1.0, 1.0, 1.0, 0.5), font);

        let mut footer = Row::new(
            5.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::SpaceBetween,
        );
        footer.add_widget(version);
        footer.add_widget(license);

        let mut container = Column::new(
            50.0,
            Alignment::Center,
            Vec4::new(20.0, 20.0, 60.0, 20.0),
            Justification::SpaceBetween,
        );

        container.add_widget(header);
        container.add_widget(buttons);
        container.add_widget(footer);

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
        });

        Self {
            container,
            font: Rc::clone(font),
            texture: gui_tex,
        }
    }
}

impl super::Scene for TitleScreen {
    fn handle_event(&mut self, _gl: &std::sync::Arc<glow::Context>, event: &sdl2::event::Event) {
        if let sdl2::event::Event::Window { win_event, .. } = event
            && let sdl2::event::WindowEvent::Resized(width, _) = win_event
        {
            let container_padding_left_right = self.container.padding.x + self.container.padding.y;
            self.container.get_widget_mut::<Row>(2).unwrap().min_size =
                Vec2::new(*width as f32 - container_padding_left_right, 0.0);

            if *width >= 1050 {
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
                    .size = Vec2::new(*width as f32 - 40.0, 80.0);
                self.container
                    .find_widget_mut::<Button>(&[1, 1, 0])
                    .unwrap()
                    .size = Vec2::new((*width as f32 - 40.0 - 5.0) / 2.0, 80.0);
                self.container
                    .find_widget_mut::<Button>(&[1, 1, 1])
                    .unwrap()
                    .size = Vec2::new((*width as f32 - 40.0 - 5.0) / 2.0, 80.0);
            }
        }
    }

    fn update(
        &mut self,
        _gl: &Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        window: &mut sdl2::video::Window,
        sdl_ctx: &sdl2::Sdl,
    ) -> super::SceneSwitch {
        window.set_title("Mineplace3D").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);
        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
        });

        if self
            .container
            .find_widget::<Button>(&[1, 0])
            .is_some_and(|btn| btn.is_released())
        {
            return super::SceneSwitch::Push(Box::new(
                crate::scenes::worldcreation::WorldCreation::new(
                    &self.font,
                    self.texture,
                    window.size(),
                ),
            ));
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 0])
            .is_some_and(|btn| btn.is_released())
        {
            // Options button pressed
            // Right now we do nothing
            println!("Options");
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 1])
            .is_some_and(|btn| btn.is_released())
        {
            return super::SceneSwitch::Quit;
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
