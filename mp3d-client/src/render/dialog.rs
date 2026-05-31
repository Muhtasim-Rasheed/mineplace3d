use glam::{FloatExt, Vec2, Vec4};

use crate::{
    render::ui::{
        uirenderer::UIRenderer,
        widgets::{Dialog, LayoutContext, Widget},
    },
    scenes::Assets,
};

pub fn draw_dialog(text: &str, assets: &Assets, ui: &mut UIRenderer, time: f32, time_start: f32) {
    // Animation is 6 seconds long
    let t = time - time_start;
    if (0.0..6.0).contains(&t) {
        let x = dialog_animation_x_at(t);
        // log::debug!("{}", x);

        let mut dialog = Dialog::new(text, Vec4::ONE, 24.0, 600.0);
        let layout_ctx = LayoutContext {
            max_size: Vec2::INFINITY,
            cursor: Vec2::new(x, 20.0),
            assets,
        };
        dialog.layout(&layout_ctx);
        dialog.draw(ui, assets);
    }
}

fn dialog_animation_x_at(t: f32) -> f32 {
    // enter - 0.0..1.0
    // stay - 1.0..5.0
    // exit - 5.0..6.0

    if t < 1.0 {
        let p = (1.0 - t).powf(3.0);
        20.0.lerp(-600.0, p)
    } else if t < 5.0 {
        20.0
    } else if t < 6.0 {
        let t = t - 5.0;
        let p = t.powf(3.0);
        20.0.lerp(-600.0, p)
    } else {
        -600.0
    }
}
