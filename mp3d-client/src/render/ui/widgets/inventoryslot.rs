use std::{cell::RefCell, rc::Rc};

use glam::{Mat4, UVec2, UVec4, Vec2, Vec4};
use mp3d_core::item::*;

use crate::{
    abs::TextureHandle,
    client::player::ClientInventory,
    render::ui::{
        uirenderer::DrawCommand,
        widgets::{Font, NineSlice, Widget},
    },
};

pub const INVENTORY_SLOT_SIZE: Vec2 = Vec2::new(64.0, 64.0);

pub struct InventorySlot {
    position: Vec2,
    nineslice: NineSlice,
    inventory: Rc<RefCell<ClientInventory>>,
    font: Rc<Font>,
    idx: usize,
}

impl InventorySlot {
    pub fn new(
        texture: TextureHandle,
        font: &Rc<Font>,
        inventory: &Rc<RefCell<ClientInventory>>,
        idx: usize,
    ) -> Self {
        let nineslice = NineSlice::new(
            texture,
            [UVec2::new(16, 16),
            UVec2::new(16, 16)],
            INVENTORY_SLOT_SIZE,
            UVec4::splat(1),
            4,
            1,
            Vec4::ONE,
        );
        Self {
            position: Vec2::ZERO,
            nineslice,
            inventory: Rc::clone(inventory),
            font: Rc::clone(font),
            idx,
        }
    }

    pub fn draw_stack(
        stack: ItemStack,
        assets: &crate::scenes::Assets,
        position: Vec2,
        ui: &super::UIRenderer,
        font: &Rc<Font>,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        let item = stack.item;
        if let Some(block) = item.assoc_block {
            if block.visible {
                let item_block_state =
                    mp3d_core::block::BlockState::default_state(block.state_type).unwrap();
                let item_block_model = assets
                    .block_models
                    .get(&(block.ident, item_block_state.to_ident().unwrap()))
                    .unwrap();
                commands.extend(item_block_model.draw_commands(
                    &ui.gl,
                    &assets.block_textures,
                    position,
                    INVENTORY_SLOT_SIZE / 1.75,
                    Mat4::from_rotation_z(180f32.to_radians())
                        * Mat4::from_rotation_x(30f32.to_radians())
                        * Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_4),
                ));
            }
        } else {
            todo!("Implement item rendering for non-block items");
        }
        // Draw the item count if greater than 1
        if stack.count > 1 {
            let bottom_right = position + INVENTORY_SLOT_SIZE / 2.0;
            let count_text = stack.count.to_string();
            let text_position =
                bottom_right - font.measure_text(&count_text, 24.0) - Vec2::new(4.0, 4.0);
            let text_commands = font
                .text(&count_text, 24.0, Vec4::ONE)
                .into_iter()
                .map(|mut cmd| {
                    if let DrawCommand::Quad { rect, .. } = &mut cmd {
                        rect[0] += text_position;
                        rect[1] += text_position;
                    } else if let DrawCommand::Mesh { vertices, .. } = &mut cmd {
                        for vertex in vertices {
                            vertex.position += text_position.extend(0.0);
                        }
                    }
                    cmd
                })
                .collect::<Vec<_>>();
            commands.extend(text_commands);
        }
        commands
    }
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
        let right = ctx.mouse.pressed.contains(&sdl2::mouse::MouseButton::Right);
        let clicked = right || ctx.mouse.pressed.contains(&sdl2::mouse::MouseButton::Left);
        let mouse_pos = ctx.mouse.position;
        let slot_pos = self.position;
        let slot_size = INVENTORY_SLOT_SIZE;
        if mouse_pos.x >= slot_pos.x
            && mouse_pos.x <= slot_pos.x + slot_size.x
            && mouse_pos.y >= slot_pos.y
            && mouse_pos.y <= slot_pos.y + slot_size.y
        {
            if clicked {
                let mut inventory = self.inventory.borrow_mut();
                inventory.click(self.idx, right);
            }
            self.nineslice.tint = Vec4::new(1.1, 1.1, 1.1, 1.0);
        } else {
            self.nineslice.tint = Vec4::ONE;
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
        if let Some(item_stack) = inventory.inner.main.get(self.idx) {
            let commands = Self::draw_stack(
                *item_stack,
                assets,
                self.position + INVENTORY_SLOT_SIZE / 2.0,
                ui_renderer,
                &self.font,
            );
            for cmd in commands {
                ui_renderer.add_command(cmd);
            }
        }
    }
}
