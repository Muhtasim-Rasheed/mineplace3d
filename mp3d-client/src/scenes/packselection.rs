use std::sync::{Arc, RwLock};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::Assets,
};

pub struct PackSelection {
    container: Column,
    available_selected: Option<usize>,
    using_selected: Option<usize>,
    available_packs: Vec<String>,
}

impl PackSelection {
    pub fn new(loaded_packs: &[String], assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let header = Label::new("Resource Packs", 48.0, Vec4::ONE);

        let mut available_column = Column::new(
            10.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            Some(window_size.1 as f32 - 200.0),
        );

        for pack in Self::get_packs() {
            let label = Button::new(&pack, Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
            available_column.add_widget(label);
        }

        let mut using_column = Column::new(
            10.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            Some(window_size.1 as f32 - 200.0),
        );

        let mut base_button = Button::new("Base Game", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
        base_button.disabled = true;
        using_column.add_widget(base_button);

        for pack in loaded_packs {
            let label = Button::new(pack, Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
            using_column.add_widget(label);
        }

        let add_button = Button::new("Add >", Vec4::ONE, 24.0, Vec2::new(250.0, 70.0));
        let remove_button = Button::new("< Remove", Vec4::ONE, 24.0, Vec2::new(250.0, 70.0));
        let up_button = Button::new("Move Up", Vec4::ONE, 24.0, Vec2::new(250.0, 70.0));
        let down_button = Button::new("Move Down", Vec4::ONE, 24.0, Vec2::new(250.0, 70.0));

        let mut buttons = Column::new(
            10.0,
            Alignment::Center,
            Vec4::ZERO,
            Justification::Start,
            None,
        );
        buttons.add_widget(add_button);
        buttons.add_widget(remove_button);
        buttons.add_widget(up_button);
        buttons.add_widget(down_button);

        let mut lists_container =
            Row::new(20.0, Alignment::Start, Vec4::ZERO, Justification::Start);
        lists_container.add_widget(available_column);
        lists_container.add_widget(buttons);
        lists_container.add_widget(using_column);

        let done_button = Button::new("Done", Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));

        let mut container = Column::new(
            30.0,
            Alignment::Center,
            Vec4::new(0.0, 0.0, 40.0, 60.0),
            Justification::Start,
            None,
        );
        container.add_widget(header);
        container.add_widget(lists_container);
        container.add_widget(done_button);

        container.layout(&LayoutContext {
            max_size: Vec2::new(window_size.0 as f32, window_size.1 as f32),
            cursor: Vec2::ZERO,
            assets,
        });

        Self {
            container,
            available_selected: None,
            using_selected: None,
            available_packs: Self::get_packs(),
        }
    }

    fn get_packs() -> Vec<String> {
        let resource_packs_dir = crate::get_resource_packs_dir();
        if let Ok(entries) = std::fs::read_dir(resource_packs_dir) {
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

impl super::Scene for PackSelection {
    fn update(
        &mut self,
        _gl: &Arc<glow::Context>,
        ctx: &crate::other::UpdateContext,
        _window: &mut sdl2::video::Window,
        _sdl_ctx: &sdl2::Sdl,
        assets: &Arc<Assets>,
        config: &Arc<RwLock<super::ClientConfig>>,
    ) -> super::SceneAction {
        let new_available_packs = Self::get_packs();
        if self.available_packs != new_available_packs {
            self.available_packs = new_available_packs;
            let available_column = self.container.find_widget_mut::<Column>(&[1, 0]).unwrap();
            available_column.widgets.clear();
            for pack in &self.available_packs {
                let label = Button::new(pack, Vec4::ONE, 24.0, Vec2::new(500.0, 80.0));
                available_column.add_widget(label);
            }
        }

        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(_window.size().0 as f32, _window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets: assets,
        });

        let available_len = self
            .container
            .find_widget::<Column>(&[1, 0])
            .unwrap()
            .widgets
            .len();

        for i in 0..available_len {
            let button = self
                .container
                .find_widget_mut::<Button>(&[1, 0, i])
                .unwrap();

            button.disabled = config
                .read()
                .unwrap()
                .resource_packs
                .as_ref()
                .map_or(false, |packs| packs.contains(&button.label));

            if button.is_released() {
                self.available_selected = Some(i);
                self.using_selected = None;
            }

            button.always_hovered = Some(i) == self.available_selected;
        }

        let using_len = self
            .container
            .find_widget::<Column>(&[1, 2])
            .unwrap()
            .widgets
            .len();

        for i in 1..using_len {
            let button = self
                .container
                .find_widget_mut::<Button>(&[1, 2, i])
                .unwrap();

            if button.is_released() {
                self.using_selected = Some(i);
                self.available_selected = None;
            }

            button.always_hovered = Some(i) == self.using_selected;
        }

        self.container
            .find_widget_mut::<Button>(&[1, 1, 0])
            .unwrap()
            .disabled = self.available_selected.is_none();
        self.container
            .find_widget_mut::<Button>(&[1, 1, 1])
            .unwrap()
            .disabled = self.using_selected.is_none();
        self.container
            .find_widget_mut::<Button>(&[1, 1, 2])
            .unwrap()
            .disabled = self.using_selected.is_none();
        self.container
            .find_widget_mut::<Button>(&[1, 1, 3])
            .unwrap()
            .disabled = self.using_selected.is_none();

        if self
            .container
            .find_widget::<Button>(&[1, 1, 0])
            .unwrap()
            .is_released()
        {
            if let Some(selected) = self.available_selected {
                let button = self
                    .container
                    .find_widget_mut::<Button>(&[1, 0, selected])
                    .unwrap();
                let pack_name = button.label.clone();
                let mut guard = config.write().unwrap();
                let resource_packs = guard.resource_packs.get_or_insert_default();
                if !resource_packs.contains(&pack_name) {
                    resource_packs.push(pack_name.clone());
                    self.container
                        .find_widget_mut::<Column>(&[1, 2])
                        .unwrap()
                        .add_widget(Button::new(
                            &pack_name,
                            Vec4::ONE,
                            24.0,
                            Vec2::new(500.0, 80.0),
                        ));
                    self.available_selected = None;
                }
            }
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 1])
            .unwrap()
            .is_released()
        {
            if let Some(selected) = self.using_selected {
                let button = self
                    .container
                    .find_widget_mut::<Button>(&[1, 2, selected])
                    .unwrap();
                let pack_name = button.label.clone();
                let mut guard = config.write().unwrap();
                if let Some(resource_packs) = guard.resource_packs.as_mut() {
                    if let Some(pos) = resource_packs.iter().position(|x| x == &pack_name) {
                        resource_packs.remove(pos);
                        let using_column =
                            self.container.find_widget_mut::<Column>(&[1, 2]).unwrap();
                        using_column.widgets.remove(selected);
                        self.using_selected = None;
                    }
                }
            }
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 2])
            .unwrap()
            .is_released()
        {
            if let Some(selected) = self.using_selected {
                if selected > 1 {
                    self.container
                        .find_widget_mut::<Column>(&[1, 2])
                        .unwrap()
                        .widgets
                        .swap(selected, selected - 1);
                    let mut guard = config.write().unwrap();
                    if let Some(resource_packs) = guard.resource_packs.as_mut() {
                        resource_packs.swap(selected - 2, selected - 1);
                    }
                    self.using_selected = Some(selected - 1);
                }
            }
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 3])
            .unwrap()
            .is_released()
        {
            if let Some(selected) = self.using_selected {
                let using_len = self
                    .container
                    .find_widget::<Column>(&[1, 2])
                    .unwrap()
                    .widgets
                    .len();
                if selected < using_len - 1 {
                    self.container
                        .find_widget_mut::<Column>(&[1, 2])
                        .unwrap()
                        .widgets
                        .swap(selected, selected + 1);
                    let mut guard = config.write().unwrap();
                    if let Some(resource_packs) = guard.resource_packs.as_mut() {
                        resource_packs.swap(selected - 1, selected);
                    }
                    self.using_selected = Some(selected + 1);
                }
            }
        }

        if self
            .container
            .get_widget::<Button>(2)
            .unwrap()
            .is_released()
        {
            config.read().unwrap().save();

            return super::SceneAction::ReloadAssetsAndPop;
        }

        super::SceneAction::None
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
