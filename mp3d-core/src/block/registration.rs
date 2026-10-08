use std::borrow::Cow;

use fxhash::FxHashMap;
use glam::{IVec3, Vec3};

use crate::{
    block::{
        BlockState, CollisionShape,
        blockstate::{HorizontalDir, PropertyValue, SlabHalf},
    },
    direction::Direction,
    entity::EntityId,
    registry::{Def, DefId, LazyId, Registry, RegistryToken},
    world::World,
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

impl serde::Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(block_registry().get(*self).unwrap().ident)
    }
}

impl<'de> serde::Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match block_registry().get_id(&s) {
            Some(id) => Ok(id),
            None => Err(serde::de::Error::custom("unknown block identifier")),
        }
    }
}

pub type OnClick =
    Box<dyn Fn(&mut World, EntityId, IVec3, BlockState, Direction) -> bool + Send + Sync>;
pub type OnPlace =
    Box<dyn Fn(BlockId, &mut World, EntityId, IVec3, Direction) -> BlockState + Send + Sync>;
pub type OnBreak = Box<dyn Fn(&mut World, EntityId, IVec3, BlockState) + Send + Sync>;
pub type OnUpdate = Box<dyn Fn(&mut World, IVec3, BlockState) -> bool + Send + Sync>;

pub struct PropertyDef {
    pub name: &'static str,
    pub bits: u8,
    pub default: u128,
    pub value_name: fn(u128) -> Cow<'static, str>,
    pub parse: fn(&str) -> Option<u128>,
    pub count: u128,
    pub shift: u8,
    pub mask: u128,
    type_id: std::any::TypeId,
}

impl PropertyDef {
    pub fn new<T: PropertyValue + 'static>(name: &'static str, default: T) -> Self {
        Self {
            name,
            bits: (128 - (T::COUNT - 1).leading_zeros()) as u8,
            default: default.to_index(),
            value_name: |i| T::from_index(i).name(),
            parse: |s| T::parse(s).map(T::to_index),
            count: T::COUNT,
            shift: 0,
            mask: 0,
            type_id: std::any::TypeId::of::<T>(),
        }
    }

    #[inline]
    pub fn get(&self, data: u128) -> u128 {
        (data >> self.shift) & self.mask
    }

    #[inline]
    pub fn set(&self, data: &mut u128, v: u128) {
        *data = (*data & !(self.mask << self.shift)) | ((v & self.mask) << self.shift);
    }

    #[inline]
    pub fn check<T: PropertyValue + 'static>(&self) -> bool {
        self.type_id == std::any::TypeId::of::<T>()
    }
}

pub struct BlockDef {
    pub visible: bool,
    pub collision_shape: CollisionShape,
    pub interact_shape: Option<CollisionShape>,
    pub ident: &'static str,

    pub state_properties: Vec<PropertyDef>,
    pub state_index: FxHashMap<&'static str, usize>,
    pub default_state: u128,
    pub from_legacy_state: Option<fn(BlockState, u16) -> BlockState>,

    pub on_click: Option<OnClick>,
    pub on_place: Option<OnPlace>,
    pub on_break: Option<OnBreak>,

    pub on_update: Option<OnUpdate>,
}

impl Def for BlockDef {
    type Id = BlockId;
    fn ident(&self) -> &'static str {
        self.ident
    }
}

impl BlockDef {
    pub fn property(&self, name: &str) -> Option<&PropertyDef> {
        self.state_index
            .get(name)
            .map(|&i| &self.state_properties[i])
    }

    pub fn all_state_data(&self) -> Vec<u128> {
        let mut all = vec![0u128];
        for p in &self.state_properties {
            all = all
                .iter()
                .flat_map(|&base| (0..p.count).map(move |v| base | (v << p.shift)))
                .collect();
        }
        all
    }

    fn finalize(&mut self) {
        let mut shift = 0u32;
        for (i, d) in self.state_properties.iter_mut().enumerate() {
            assert!(
                shift + d.bits as u32 <= 128,
                "block {} needs more than 128 bits of properties",
                self.ident
            );
            d.shift = shift as u8;
            d.mask = if d.bits == 128 {
                u128::MAX
            } else {
                (1u128 << d.bits) - 1
            };
            self.default_state |= d.default << d.shift;
            self.state_index.insert(d.name, i);
            shift += d.bits as u32;
        }
    }
}

pub type BlockRegistry = Registry<BlockDef>;

static BLOCK_REGISTRY: std::sync::OnceLock<BlockRegistry> = std::sync::OnceLock::new();

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
        let mut def = (reg.build)();
        def.finalize();
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

                $(, state_properties: $state_properties:expr)?
                $(, from_legacy_state: $from_legacy_state:expr)?

                $(, on_click: $on_click:expr)?
                $(, on_place: $on_place:expr)?
                $(, on_break: $on_break:expr)?

                $(, on_update: $on_update:expr)?
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

                            state_properties: define_blocks!(@state_properties $( $state_properties )?),
                            state_index: ::fxhash::FxHashMap::default(),
                            default_state: 0,
                            from_legacy_state: define_blocks!(@from_legacy_state $( $from_legacy_state )?),

                            on_click: define_blocks!(@on_click $( $on_click )?),
                            on_place: define_blocks!(@on_place $( $on_place )?),
                            on_break: define_blocks!(@on_break $( $on_break )?),

                            on_update: define_blocks!(@on_update $( $on_update )?),
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

    (@state_properties $state_properties:expr) => { $state_properties };
    (@state_properties) => { vec![] };

    (@from_legacy_state $from_legacy_state:expr) => { Some($from_legacy_state) };
    (@from_legacy_state) => { None };

    (@on_click $on_click:expr) => { Some($on_click) };
    (@on_click) => { None };

    (@on_place $on_place:expr) => { Some($on_place) };
    (@on_place) => { None };

    (@on_break $on_break:expr) => { Some($on_break) };
    (@on_break) => { None };

    (@on_update $on_update:expr) => { Some($on_update) };
    (@on_update) => { None };
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
                let Some(shape) = block_state.get::<SlabHalf>("half") else {
                    return false;
                };
                let (block_min, block_max) = match shape {
                    SlabHalf::Bottom => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0)),
                    SlabHalf::Top => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                    SlabHalf::Both => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                };
                crate::aabb_overlap(player_min, player_max, block_min, block_max)
            }
            CollisionShape::Stairs => {
                let Some(HorizontalDir(shape)) = block_state.get::<HorizontalDir>("facing") else {
                    return false;
                };
                let element_a_min = Vec3::new(0.0, 0.0, 0.0);
                let element_a_max = Vec3::new(1.0, 0.5, 1.0);
                let (element_b_min, element_b_max) = match shape {
                    Direction::North => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 1.0, 0.5)),
                    Direction::South => (Vec3::new(0.0, 0.5, 0.5), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::East => (Vec3::new(0.5, 0.5, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::West => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.5, 1.0, 1.0)),
                    _ => unreachable!(),
                };
                crate::aabb_overlap(player_min, player_max, element_a_min, element_a_max)
                    || crate::aabb_overlap(player_min, player_max, element_b_min, element_b_max)
            }
            CollisionShape::VSlab => {
                let Some(HorizontalDir(shape)) = block_state.get::<HorizontalDir>("facing") else {
                    return false;
                };
                let (block_min, block_max) = match shape {
                    Direction::North => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.5)),
                    Direction::South => (Vec3::new(0.0, 0.0, 0.5), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::East => (Vec3::new(0.5, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::West => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 1.0, 1.0)),
                    _ => unreachable!(),
                };
                crate::aabb_overlap(player_min, player_max, block_min, block_max)
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
                let Some(shape) = block_state.get::<SlabHalf>("half") else {
                    return None;
                };
                let (block_min, block_max) = match shape {
                    SlabHalf::Bottom => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0)),
                    SlabHalf::Top => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                    SlabHalf::Both => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                };
                crate::ray_intersect_aabb(
                    ray_origin_local,
                    ray_direction_local,
                    block_min,
                    block_max,
                )
            }
            CollisionShape::Stairs => {
                let Some(HorizontalDir(shape)) = block_state.get::<HorizontalDir>("facing") else {
                    return None;
                };
                let element_a_min = Vec3::new(0.0, 0.0, 0.0);
                let element_a_max = Vec3::new(1.0, 0.5, 1.0);
                let (element_b_min, element_b_max) = match shape {
                    Direction::North => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.0, 1.0, 0.5)),
                    Direction::South => (Vec3::new(0.0, 0.5, 0.5), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::East => (Vec3::new(0.5, 0.5, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::West => (Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.5, 1.0, 1.0)),
                    _ => unreachable!(),
                };

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
            }
            CollisionShape::VSlab => {
                let Some(HorizontalDir(shape)) = block_state.get::<HorizontalDir>("facing") else {
                    return None;
                };
                let (block_min, block_max) = match shape {
                    Direction::North => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.5)),
                    Direction::South => (Vec3::new(0.0, 0.0, 0.5), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::East => (Vec3::new(0.5, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                    Direction::West => (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 1.0, 1.0)),
                    _ => unreachable!(),
                };
                crate::ray_intersect_aabb(
                    ray_origin_local,
                    ray_direction_local,
                    block_min,
                    block_max,
                )
            }
        }
    }
}
