use std::sync::{Arc, RwLock};

use glam::{Vec2, Vec4};
use glow::HasContext;

use crate::{
    render::ui::{uirenderer::UIRenderer, widgets::*},
    scenes::{Assets, SceneAction, SceneUpdateContext},
};

pub struct PackSelection {
    container: Column,
    available_selected: Option<usize>,
    using_selected: Option<usize>,
    available_packs: Vec<String>,
}

impl PackSelection {
    pub fn new(loaded_packs: &[String], assets: &Arc<Assets>, window_size: (u32, u32)) -> Self {
        let mut container = Column::new(30.0)
            .padding(Vec4::new(0.0, 0.0, 40.0, 60.0))
            .with(Label::new("Resource Packs").font_size(48.0))
            .with(
                Row::new(20.0)
                    .alignment(Alignment::Start)
                    .with(
                        Column::new(10.0)
                            .viewport_height(window_size.1 as f32 - 200.0)
                            .with_many(Self::get_packs().into_iter().map(|v| Button::new(&v))),
                    )
                    .with(
                        Column::new(10.0)
                            .with(Button::new("Add >").size(Vec2::new(250.0, 70.0)))
                            .with(Button::new("< Remove").size(Vec2::new(250.0, 70.0)))
                            .with(Button::new("Move Up").size(Vec2::new(250.0, 70.0)))
                            .with(Button::new("Move Down").size(Vec2::new(250.0, 70.0))),
                    )
                    .with(
                        Column::new(10.0)
                            .viewport_height(window_size.1 as f32 - 200.0)
                            .with(Button::new("Base Game").disabled())
                            .with_many(loaded_packs.into_iter().map(|v| Button::new(&v))),
                    ),
            )
            .with(Button::new("Done").size(Vec2::new(250.0, 70.0)));

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
    fn update(&mut self, ctx: &mut SceneUpdateContext) -> Vec<SceneAction> {
        let SceneUpdateContext {
            ctx,
            window,
            sdl_ctx,
            assets,
            config,
            ..
        } = ctx;

        window.set_title("Mineplace3D - Resource packs").unwrap();
        sdl_ctx.mouse().set_relative_mouse_mode(false);

        let new_available_packs = Self::get_packs();
        if self.available_packs != new_available_packs {
            self.available_packs = new_available_packs;
            let available_column = self.container.find_widget_mut::<Column>(&[1, 0]).unwrap();
            available_column.widgets.clear();
            for pack in &self.available_packs {
                let label = Button::new(pack);
                available_column.add_widget(label);
            }
        }

        self.container.update(ctx);
        self.container.layout(&LayoutContext {
            max_size: Vec2::new(window.size().0 as f32, window.size().1 as f32),
            cursor: Vec2::ZERO,
            assets,
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
                .is_some_and(|packs| packs.contains(&button.text));

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
            && let Some(selected) = self.available_selected
        {
            let button = self
                .container
                .find_widget_mut::<Button>(&[1, 0, selected])
                .unwrap();
            let pack_name = button.text.clone();
            let mut guard = config.write().unwrap();
            let resource_packs = guard.resource_packs.get_or_insert_default();
            if !resource_packs.contains(&pack_name) {
                resource_packs.push(pack_name.clone());
                self.container
                    .find_widget_mut::<Column>(&[1, 2])
                    .unwrap()
                    .add_widget(Button::new(&pack_name));
                self.available_selected = None;
            }
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 1])
            .unwrap()
            .is_released()
            && let Some(selected) = self.using_selected
        {
            let button = self
                .container
                .find_widget_mut::<Button>(&[1, 2, selected])
                .unwrap();
            let pack_name = button.text.clone();
            let mut guard = config.write().unwrap();
            if let Some(resource_packs) = guard.resource_packs.as_mut()
                && let Some(pos) = resource_packs.iter().position(|x| x == &pack_name)
            {
                resource_packs.remove(pos);
                let using_column = self.container.find_widget_mut::<Column>(&[1, 2]).unwrap();
                using_column.widgets.remove(selected);
                self.using_selected = None;
            }
        }

        if self
            .container
            .find_widget::<Button>(&[1, 1, 2])
            .unwrap()
            .is_released()
            && let Some(selected) = self.using_selected
            && selected > 1
        {
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

        if self
            .container
            .find_widget::<Button>(&[1, 1, 3])
            .unwrap()
            .is_released()
            && let Some(selected) = self.using_selected
        {
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

        if self
            .container
            .get_widget::<Button>(2)
            .unwrap()
            .is_released()
        {
            config.read().unwrap().save();

            return vec![SceneAction::ReloadAssets, SceneAction::Pop];
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
