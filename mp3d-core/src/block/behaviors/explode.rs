use glam::IVec3;

use crate::{
    block::{BlockState, blocks},
    direction::Direction,
    entity::EntityId,
    protocol::BlockUpdateKind,
    world::World,
};

pub fn on_click(
    world: &mut World,
    _: EntityId,
    block_pos: IVec3,
    _: BlockState,
    _: Direction,
) -> bool {
    let radius_sq = 8 * 8;
    for x in -8..=8 {
        for y in -8..=8 {
            for z in -8..=8 {
                if x * x + y * y + z * z <= radius_sq {
                    let pos = block_pos + IVec3::new(x, y, z);
                    world.urgent_set_block_at(
                        pos,
                        BlockState::default_for(*blocks::AIR),
                        BlockUpdateKind::Interaction,
                    );
                }
            }
        }
    }
    true
}
