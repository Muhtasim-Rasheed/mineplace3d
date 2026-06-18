use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, blocks},
    direction::Direction,
    protocol::BlockUpdateKind,
    world::World,
};

pub fn on_click(
    _: BlockId,
    world: &mut World,
    _: u64,
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
                        *blocks::AIR,
                        BlockState::none(),
                        BlockUpdateKind::Interaction,
                    );
                }
            }
        }
    }
    true
}
