use std::{cell::RefCell, rc::Rc};

use glam::{Mat4, UVec2, UVec4, Vec2, Vec4};
use mp3d_core::item::*;

use crate::{
    abs::{Texture, TextureHandle},
    render::ui::{
        uirenderer::DrawCommand,
        widgets::{NineSlice, Stack, Widget},
    },
};

pub const INVENTORY_SLOT_SIZE: Vec2 = Vec2::new(64.0, 64.0);

pub struct InventorySlot {
    position: Vec2,
    nineslice: NineSlice,
    inventory: Rc<RefCell<Inventory>>,
    idx: usize,
}

impl InventorySlot {
    pub fn new(texture: TextureHandle, inventory: &Rc<RefCell<Inventory>>, idx: usize) -> Self {
        let nineslice = NineSlice::new(
            texture,
            UVec2::new(16, 16),
            UVec2::new(16, 16),
            INVENTORY_SLOT_SIZE,
            UVec4::splat(1),
            4,
            1,
            Vec4::ONE,
        );
        let mut slot = Self {
            position: Vec2::ZERO,
            nineslice,
            inventory: Rc::clone(inventory),
            idx,
        };
        slot.setup_stack();
        slot
    }

    fn setup_stack(&mut self) {}
}

impl Widget for InventorySlot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        INVENTORY_SLOT_SIZE
    }

    fn update(&mut self, ctx: &crate::other::UpdateContext) {
        if ctx.mouse.pressed.contains(&sdl2::mouse::MouseButton::Left) {
            let mouse_pos = ctx.mouse.position;
            let slot_pos = self.position;
            let slot_size = INVENTORY_SLOT_SIZE;
            if mouse_pos.x >= slot_pos.x
                && mouse_pos.x <= slot_pos.x + slot_size.x
                && mouse_pos.y >= slot_pos.y
                && mouse_pos.y <= slot_pos.y + slot_size.y
            {
                let mut inventory = self.inventory.borrow_mut();
                // TODO: Handle inventory interactions.
            }
        }
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint().min(ctx.max_size);
        self.position = ctx.cursor;
        let layout_ctx = super::LayoutContext {
            max_size: measured_size,
            cursor: self.position,
        };
        self.nineslice.layout(&layout_ctx);
        Vec2::new(
            measured_size.x.min(ctx.max_size.x),
            measured_size.y.min(ctx.max_size.y),
        )
    }

    fn draw(
        &self,
        ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer,
        assets: &crate::scenes::Assets,
    ) {
        self.nineslice.draw(ui_renderer, assets);

        let inventory = self.inventory.borrow();
        if let Some(item_stack) = inventory.main.get(self.idx) {
            let item = item_stack.item;
            if let Some(block) = item.assoc_block {
                if block.visible {
                    let item_block_state =
                        mp3d_core::block::BlockState::default_state(block.state_type).unwrap();
                    let item_block_model = assets
                        .block_models
                        .get(&(block.ident, item_block_state.to_ident().unwrap()))
                        .unwrap();
                    let commands = item_block_model.draw_commands(
                        &ui_renderer.gl,
                        &assets.block_textures,
                        self.position + INVENTORY_SLOT_SIZE / 2.0,
                        INVENTORY_SLOT_SIZE / 1.75,
                        Mat4::from_rotation_z(180f32.to_radians())
                            * Mat4::from_rotation_x(30f32.to_radians())
                            * Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_4),
                    );
                    for cmd in commands {
                        ui_renderer.add_command(cmd);
                    }
                }
            } else {
                todo!("Implement item rendering for non-block items");
            }
        }
    }
}
