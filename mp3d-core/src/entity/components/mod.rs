use glam::Vec3;

use crate::{
    define_component,
    entity::{
        EntityId, MoveInput,
        registration::{EntityDefId, entity_registry},
    },
    item::Inventory,
    serialize::{
        read::{ByteReader, ReadError},
        write::ByteWriter,
    },
};
use std::any::Any;

pub use registration::*;
pub mod registration;

pub trait Component: Any + Send + Sync + 'static {
    fn component_id() -> ComponentId
    where
        Self: Sized;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct ComponentStorage<T> {
    dense: Vec<T>,
    dense_to_entity: Vec<EntityId>,
    sparse: Vec<Option<u32>>,
}

impl<T> ComponentStorage<T> {
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            dense_to_entity: Vec::new(),
            sparse: Vec::new(),
        }
    }

    fn sparse_idx(&self, entity: EntityId) -> usize {
        entity.index as usize
    }

    pub fn insert(&mut self, entity: EntityId, component: T) {
        let idx = self.sparse_idx(entity);
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }
        if let Some(dense_idx) = self.sparse[idx] {
            self.dense[dense_idx as usize] = component;
        } else {
            self.sparse[idx] = Some(self.dense.len() as u32);
            self.dense.push(component);
            self.dense_to_entity.push(entity);
        }
    }

    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let idx = self.sparse_idx(entity);
        let dense_idx = *self.sparse.get(idx)?;
        dense_idx.map(|i| &self.dense[i as usize])
    }

    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let idx = self.sparse_idx(entity);
        let dense_idx = *self.sparse.get(idx)?;
        dense_idx.map(move |i| &mut self.dense[i as usize])
    }

    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let idx = self.sparse_idx(entity);
        let dense_idx = self.sparse.get(idx).copied().flatten()? as usize;

        let removed = self.dense.swap_remove(dense_idx);
        self.dense_to_entity.swap_remove(dense_idx);
        self.sparse[idx] = None;

        if dense_idx < self.dense.len() {
            let moved_entity = self.dense_to_entity[dense_idx];
            let sparse_idx = self.sparse_idx(moved_entity);
            self.sparse[sparse_idx] = Some(dense_idx as u32);
        }

        Some(removed)
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.dense_to_entity.iter().copied().zip(self.dense.iter())
    }
}

pub trait ErasedStorage: Any {
    fn remove(&mut self, entity: EntityId);
    fn snapshot(&self, entity: EntityId) -> Option<Vec<u8>>;
    fn apply(&mut self, entity: EntityId, bytes: &[u8]) -> Result<(), ReadError>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> ErasedStorage for ComponentStorage<T> {
    fn remove(&mut self, entity: EntityId) {
        self.remove(entity);
    }
    fn snapshot(&self, entity: EntityId) -> Option<Vec<u8>> {
        self.get(entity).map(|c| c.to_bytes())
    }
    fn apply(&mut self, entity: EntityId, bytes: &[u8]) -> Result<(), ReadError> {
        self.insert(entity, T::from_bytes(bytes)?);
        Ok(())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// === Definition of all components ===

#[derive(Debug, Clone, Copy)]
pub struct Position(pub Vec3);

impl Position {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().vec3(self.0).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes).vec3().map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Velocity(pub Vec3);

impl Velocity {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().vec3(self.0).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes).vec3().map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}

impl Rotation {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().f32(self.yaw).f32(self.pitch).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);
        Ok(Self {
            yaw: reader.f32()?,
            pitch: reader.f32()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OnGround(pub bool);

impl OnGround {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().bool(self.0).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes).bool().map(Self)
    }
}

#[derive(Debug, Clone)]
pub struct Username(pub String);

impl Username {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().string(&self.0).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes).string().map(Self)
    }
}

impl Inventory {
    fn to_bytes(&self) -> Vec<u8> {
        crate::saving::Saveable::save(self)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        // TODO: convert WorldLoadError to ReadError, or maybe remove WorldLoadError in favor of
        // ReadError
        Ok(
            crate::saving::Saveable::load(&mut bytes.iter().copied(), crate::saving::SAVE_VERSION)
                .unwrap(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectedHotbarSlot(pub usize);

impl SelectedHotbarSlot {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().u8(self.0 as u8).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        match ByteReader::new(bytes).u8() {
            Ok(v) if v < 9 => Ok(Self(v as usize)),
            Ok(v) => Err(ReadError::IndexOutOfRange { value: v, max: 8 }),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flying(pub bool);

impl Flying {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().bool(self.0).into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes).bool().map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Hitbox {
    pub width: f32,
    pub height: f32,
}

impl Hitbox {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new()
            .f32(self.width)
            .f32(self.height)
            .into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);
        Ok(Self {
            width: reader.f32()?,
            height: reader.f32()?,
        })
    }
}

impl MoveInput {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new()
            .f32(self.forward)
            .f32(self.strafe)
            .bool(self.jump)
            .bool(self.sneak)
            .into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);
        Ok(Self {
            forward: reader.f32()?,
            strafe: reader.f32()?,
            jump: reader.bool()?,
            sneak: reader.bool()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HasDefId(pub EntityDefId);

impl HasDefId {
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new()
            .string(entity_registry().get(self.0).unwrap().ident)
            .into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes)
            .registry_id(entity_registry())
            .map(Self)
    }
}

define_component!(
    POSITION,
    Position,
    (Position::to_bytes, Position::from_bytes),
    "position"
);
define_component!(
    VELOCITY,
    Velocity,
    (Velocity::to_bytes, Velocity::from_bytes),
    "velocity"
);
define_component!(
    ROTATION,
    Rotation,
    (Rotation::to_bytes, Rotation::from_bytes),
    "rotation"
);
define_component!(
    ON_GROUND,
    OnGround,
    (OnGround::to_bytes, OnGround::from_bytes),
    "on_ground"
);
define_component!(
    USERNAME,
    Username,
    (Username::to_bytes, Username::from_bytes),
    "username"
);
define_component!(
    INVENTORY,
    Inventory,
    (Inventory::to_bytes, Inventory::from_bytes),
    "inventory"
);
define_component!(
    SELECTED_HOTBAR_SLOT,
    SelectedHotbarSlot,
    (SelectedHotbarSlot::to_bytes, SelectedHotbarSlot::from_bytes),
    "selected_hotbar_slot"
);
define_component!(
    FLYING,
    Flying,
    (Flying::to_bytes, Flying::from_bytes),
    "flying"
);
define_component!(
    HITBOX,
    Hitbox,
    (Hitbox::to_bytes, Hitbox::from_bytes),
    "hitbox"
);
define_component!(
    MOVE_INPUT,
    MoveInput,
    (MoveInput::to_bytes, MoveInput::from_bytes),
    "move_input"
);
define_component!(
    HAS_DEF_ID,
    HasDefId,
    (HasDefId::to_bytes, HasDefId::from_bytes),
    "has_def_id"
);
