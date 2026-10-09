use glam::IVec3;

use crate::{
    block::{BlockState, block_registry, blocks},
    direction::Direction,
    protocol::BlockUpdateKind,
    world::World,
};

pub fn on_update(world: &mut World, pos: IVec3, face: Direction) -> bool {
    let below = world.get_block_at(pos + face);
    if let Some(below) = below
        && block_registry().get(below.block).unwrap().visible
    {
        return false;
    }
    world.urgent_set_block_at(
        pos,
        BlockState::default_for(*blocks::AIR),
        BlockUpdateKind::Removed,
    );
    true
}
