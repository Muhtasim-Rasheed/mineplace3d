use glam::{IVec3, Vec3};

use crate::{
    block::{BlockState, CollisionShape},
    direction::Direction,
    registry::{Def, DefId, LazyId, Registry, RegistryToken},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

impl DefId for BlockId {
    fn new(v: usize, _token: RegistryToken) -> Self {
        Self(v)
    }

    fn get(&self) -> usize {
        self.0
    }
}

pub type OnClickHandler =
    fn(BlockId, &mut crate::world::World, u64, IVec3, BlockState, Direction) -> bool;
pub type OnPlaceHandler =
    fn(BlockId, &mut crate::world::World, u64, IVec3, Direction) -> BlockState;
pub type OnBreakHandler = fn(BlockId, &mut crate::world::World, u64, IVec3, BlockState);

pub struct BlockDef {
    pub visible: bool,
    pub collision_shape: CollisionShape,
    pub interact_shape: Option<CollisionShape>,
    pub ident: &'static str,
    pub state_type: u16,

    pub on_click: Option<OnClickHandler>,
    pub on_place: Option<OnPlaceHandler>,
    pub on_break: Option<OnBreakHandler>,
}

impl Def for BlockDef {
    type Id = BlockId;
    fn ident(&self) -> &'static str {
        self.ident
    }
}

pub type BlockRegistry = Registry<BlockDef>;

static BLOCK_REGISTRY: std::sync::OnceLock<BlockRegistry> = std::sync::OnceLock::new();

#[inline]
pub fn block_registry() -> &'static BlockRegistry {
    BLOCK_REGISTRY
        .get()
        .expect("block registry not initialized - call init_block_registry() first")
}

pub struct BlockRegistration {
    pub build: fn() -> BlockDef,
    pub id_slot: &'static LazyId<BlockId>,
}

inventory::collect!(BlockRegistration);

pub fn init_block_registry() {
    let mut registry = BlockRegistry::new();

    for reg in inventory::iter::<BlockRegistration> {
        let def = (reg.build)();
        let def_ident = def.ident;
        let id = registry
            .register(def)
            .unwrap_or_else(|e| panic!("duplicate block ident: {}", e.ident));
        reg.id_slot
            .set(id)
            .unwrap_or_else(|_| panic!("block static for {} set twice", def_ident));
    }

    BLOCK_REGISTRY
        .set(registry)
        .unwrap_or_else(|_| panic!("init_block_registry called twice"));
}

#[macro_export]
macro_rules! define_blocks {
    (
        $(
            $name:ident => {
                ident: $ident:expr
                $(, visible: $visible:expr)?
                $(, collision_shape: $collision_shape:expr)?
                $(, interact_shape: $interact_shape:expr)?
                $(, state_type: $state_type:expr)?
                $(, on_click: $on_click:expr)?
                $(, on_place: $on_place:expr)?
                $(, on_break: $on_break:expr)?
                $(,)?
            }
        ),* $(,)?
    ) => {
        pub mod blocks {
            use super::*;

            $(
                pub static $name: $crate::registry::LazyId<BlockId> = $crate::registry::LazyId::new();

                ::inventory::submit! {
                    $crate::block::BlockRegistration {
                        build: || BlockDef {
                            visible: define_blocks!(@visible $( $visible )?),
                            collision_shape: define_blocks!(@collision_shape $( $collision_shape )?),
                            interact_shape: define_blocks!(@interact_shape $( $interact_shape )?),
                            ident: $ident,
                            state_type: define_blocks!(@state_type $( $state_type )?),
                            on_click: define_blocks!(@on_click $( $on_click )?),
                            on_place: define_blocks!(@on_place $( $on_place )?),
                            on_break: define_blocks!(@on_break $( $on_break )?),
                        },
                        id_slot: &$name,
                    }
                }
            )*
        }
    };

    (@visible $visible:expr) => { $visible };
    (@visible) => { true };

    (@collision_shape $collision_shape:expr) => { $collision_shape };
    (@collision_shape) => { CollisionShape::FullBlock };

    (@interact_shape $interact_shape:expr) => { Some($interact_shape) };
    (@interact_shape) => { None };

    (@state_type $state_type:expr) => { $state_type };
    (@state_type) => { BlockState::NONE_TYPE };

    (@on_click $on_click:expr) => { Some($on_click) };
    (@on_click) => { None };

    (@on_place $on_place:expr) => { Some($on_place) };
    (@on_place) => { None };

    (@on_break $on_break:expr) => { Some($on_break) };
    (@on_break) => { None };
}

impl BlockDef {
    pub fn collides_with_player(
        &self,
        player_width: f32,
        player_height: f32,
        player_pos_local: Vec3,
        block_state: BlockState,
    ) -> bool {
        let half_width = player_width / 2.0;
        let player_min = Vec3::new(
            player_pos_local.x - half_width,
            player_pos_local.y,
            player_pos_local.z - half_width,
        );
        let player_max = Vec3::new(
            player_pos_local.x + half_width,
            player_pos_local.y + player_height,
            player_pos_local.z + half_width,
        );
        match self.collision_shape {
            CollisionShape::None => false,
            CollisionShape::FullBlock => {
                let block_min = Vec3::new(0.0, 0.0, 0.0);
                let block_max = Vec3::new(1.0, 1.0, 1.0);
                crate::aabb_overlap(player_min, player_max, block_min, block_max)
            }
            CollisionShape::Slab => {
                if let Some(shape) = block_state.is_slab() {
                    let block_min;
                    let block_max;
                    match shape {
                        0x0000 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 0.5, 1.0);
                        }
                        0x0001 => {
                            block_min = Vec3::new(0.0, 0.5, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        0x0002 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::aabb_overlap(player_min, player_max, block_min, block_max)
                } else {
                    false
                }
            }
            CollisionShape::Stairs => {
                if let Some(shape) = block_state.is_stairs() {
                    let element_a_min = Vec3::new(0.0, 0.0, 0.0);
                    let element_a_max = Vec3::new(1.0, 0.5, 1.0);
                    let element_b_min;
                    let element_b_max;
                    match shape {
                        Direction::North => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.5);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            element_b_min = Vec3::new(0.5, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::aabb_overlap(player_min, player_max, element_a_min, element_a_max)
                        || crate::aabb_overlap(player_min, player_max, element_b_min, element_b_max)
                } else {
                    false
                }
            }
            CollisionShape::VSlab => {
                if let Some(shape) = block_state.is_facing() {
                    let block_min;
                    let block_max;
                    match shape {
                        Direction::North => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            block_min = Vec3::new(0.0, 0.0, 0.5);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            block_min = Vec3::new(0.5, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::aabb_overlap(player_min, player_max, block_min, block_max)
                } else {
                    false
                }
            }
        }
    }

    /// Returns the normal of the hit face, if it hit anything.
    pub fn ray_intersect(
        &self,
        ray_origin_local: Vec3,
        ray_direction_local: Vec3,
        block_state: BlockState,
    ) -> Option<IVec3> {
        match self.interact_shape.unwrap_or(self.collision_shape) {
            CollisionShape::None => None,
            CollisionShape::FullBlock => {
                let block_min = Vec3::new(0.0, 0.0, 0.0);
                let block_max = Vec3::new(1.0, 1.0, 1.0);
                crate::ray_intersect_aabb(
                    ray_origin_local,
                    ray_direction_local,
                    block_min,
                    block_max,
                )
            }
            CollisionShape::Slab => {
                if let Some(shape) = block_state.is_slab() {
                    let block_min;
                    let block_max;
                    match shape {
                        0x0000 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 0.5, 1.0);
                        }
                        0x0001 => {
                            block_min = Vec3::new(0.0, 0.5, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        0x0002 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::ray_intersect_aabb(
                        ray_origin_local,
                        ray_direction_local,
                        block_min,
                        block_max,
                    )
                } else {
                    None
                }
            }
            CollisionShape::Stairs => {
                if let Some(shape) = block_state.is_stairs() {
                    let element_a_min = Vec3::new(0.0, 0.0, 0.0);
                    let element_a_max = Vec3::new(1.0, 0.5, 1.0);
                    let element_b_min;
                    let element_b_max;
                    match shape {
                        Direction::North => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.5);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            element_b_min = Vec3::new(0.5, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::ray_intersect_aabb(
                        ray_origin_local,
                        ray_direction_local,
                        element_a_min,
                        element_a_max,
                    )
                    .or_else(|| {
                        crate::ray_intersect_aabb(
                            ray_origin_local,
                            ray_direction_local,
                            element_b_min,
                            element_b_max,
                        )
                    })
                } else {
                    None
                }
            }
            CollisionShape::VSlab => {
                if let Some(shape) = block_state.is_facing() {
                    let block_min;
                    let block_max;
                    match shape {
                        Direction::North => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            block_min = Vec3::new(0.0, 0.0, 0.5);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            block_min = Vec3::new(0.5, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::ray_intersect_aabb(
                        ray_origin_local,
                        ray_direction_local,
                        block_min,
                        block_max,
                    )
                } else {
                    None
                }
            }
        }
    }
}
