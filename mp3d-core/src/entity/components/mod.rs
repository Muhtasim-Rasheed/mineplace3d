use glam::Vec3;

use crate::{
    entity::{
        EntityId, MoveInput,
        registration::{EntityDefId, entity_registry},
    },
    impl_component,
    item::Inventory,
    serialize::{
        read::{ByteReader, ReadError, ReadErrorExt, ReadErrorKind},
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

impl<T> Default for ComponentStorage<T> {
    fn default() -> Self {
        Self::new()
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
impl_component! {
    name: POSITION,
    ident: "position",
    ty: Position,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().vec3(self.0).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes)
            .vec3()
            .ctx("position component")
            .map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Velocity(pub Vec3);
impl_component! {
    name: VELOCITY,
    ident: "velocity",
    ty: Velocity,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().vec3(self.0).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes)
            .vec3()
            .ctx("velocity component")
            .map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}
impl_component! {
    name: ROTATION,
    ident: "rotation",
    ty: Rotation,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().f32(self.yaw).f32(self.pitch).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);
        Ok(Self {
            yaw: reader.f32().ctx("Rotation::yaw")?,
            pitch: reader.f32().ctx("Rotation::pitch")?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OnGround(pub bool);
impl_component! {
    name: ON_GROUND,
    ident: "on_ground",
    ty: OnGround,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().bool(self.0).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes)
            .bool()
            .ctx("on_ground component")
            .map(Self)
    }
}

#[derive(Debug, Clone)]
pub struct Username(pub String);
impl_component! {
    name: USERNAME,
    ident: "username",
    ty: Username,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new()
            .u16(self.0.len() as u16)
            .string(&self.0)
            .into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);
        let len = reader.u16().ctx("username length")? as usize;
        reader.string(len).ctx("username component").map(Self)
    }
}

impl_component! {
    name: INVENTORY,
    ident: "inventory",
    ty: Inventory,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().save(self).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes).load(crate::serialize::SAVE_VERSION)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectedHotbarSlot(pub usize);

impl_component! {
    name: SELECTED_HOTBAR_SLOT,
    ident: "selected_hotbar_slot",
    ty: SelectedHotbarSlot,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().u8(self.0 as u8).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        match ByteReader::new(bytes).u8() {
            Ok(v) if v < 9 => Ok(Self(v as usize)),
            Ok(v) => Err(ReadErrorKind::IndexOutOfRange { value: v, max: 8 })
                .ctx("selected_hotbar_slot component"),
            Err(e) => Err(e).ctx("selected_hotbar_slot component"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flying(pub bool);

impl_component! {
    name: FLYING,
    ident: "flying",
    ty: Flying,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new().bool(self.0).into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes)
            .bool()
            .ctx("flying component")
            .map(Self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Hitbox {
    pub width: f32,
    pub height: f32,
}
impl_component! {
    name: HITBOX,
    ident: "hitbox",
    ty: Hitbox,
    fn to_bytes(&self) -> Vec<u8> {
        ByteWriter::new()
            .f32(self.width)
            .f32(self.height)
            .into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);
        Ok(Self {
            width: reader.f32().ctx("Hitbox::width")?,
            height: reader.f32().ctx("Hitbox::height")?,
        })
    }
}

impl_component! {
    name: MOVE_INPUT,
    ident: "move_input",
    ty: MoveInput,
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
            forward: reader.f32().ctx("MoveInput::forward")?,
            strafe: reader.f32().ctx("MoveInput::strafe")?,
            jump: reader.bool().ctx("MoveInput::jump")?,
            sneak: reader.bool().ctx("MoveInput::sneak")?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HasDefId(pub EntityDefId);
impl_component! {
    name: HAS_DEF_ID,
    ident: "has_def_id",
    ty: HasDefId,
    fn to_bytes(&self) -> Vec<u8> {
        let ident = entity_registry().get(self.0).ident;
        ByteWriter::new()
            .u16(ident.len() as u16)
            .string(ident)
            .into_bytes()
    }
    fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        ByteReader::new(bytes)
            .registry_id(entity_registry())
            .ctx("has_def_id component")
            .map(Self)
    }
}
