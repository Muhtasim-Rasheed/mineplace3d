//! Game entities for Mineplace3D.
//!
//! This module provides the `Entity` trait and some implementations like the `Player` entity.

use crate::{
    define_entities,
    entity::components::{Component, ComponentId, component_registry},
    registry::Def,
    serialize::{
        read::{ByteReader, ReadError},
        write::ByteWriter,
    },
};

/// ID of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08x}-{:08x}", self.index, self.generation)
    }
}

pub struct EntityDetails {
    components: Vec<(ComponentId, Vec<u8>)>,
}

impl EntityDetails {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut reader = ByteReader::new(bytes);

        let count = reader.u16()?;
        let registry = component_registry();
        let mut components = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let ident = reader.string()?;
            let len = reader.u32()? as usize;
            let data = reader.take(len)?.to_vec();

            match registry.get_id(&ident) {
                Some(id) => components.push((id, data)),
                None => continue,
            }
        }

        Ok(Self { components })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let registry = component_registry();
        let mut writer = ByteWriter::new().u16(self.components.len() as u16);
        for (id, data) in &self.components {
            let ident = registry.get(*id).unwrap().ident();
            writer = writer.string(ident).u32(data.len() as u32).bytes(data);
        }
        writer.into_bytes()
    }

    pub fn get<T: Component>(&self) -> Option<T> {
        let id = T::component_id();
        let (_, bytes) = self.components.iter().find(|(cid, _)| *cid == id)?;
        T::from_bytes(bytes).ok()
    }

    pub fn merge(&mut self, other: &EntityDetails) {
        for (id, data) in &other.components {
            if let Some(existing) = self.components.iter_mut().find(|(eid, _)| eid == id) {
                existing.1 = data.clone();
            } else {
                self.components.push((*id, data.clone()));
            }
        }
    }

    pub fn builder() -> EntityDetailsBuilder {
        EntityDetailsBuilder {
            components: Vec::new(),
        }
    }
}

pub struct EntityDetailsBuilder {
    components: Vec<(ComponentId, Vec<u8>)>,
}

impl EntityDetailsBuilder {
    pub fn with<T: Component>(mut self, value: T) -> Self {
        self.components.push((T::component_id(), value.to_bytes()));
        self
    }

    pub fn build(self) -> EntityDetails {
        EntityDetails {
            components: self.components,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MoveInput {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub sneak: bool,
}

impl From<crate::protocol::MoveInstructions> for MoveInput {
    fn from(instr: crate::protocol::MoveInstructions) -> Self {
        Self {
            forward: match instr.forward {
                -1 => -1.0,
                0 => 0.0,
                1 => 1.0,
                2 => 1.5,
                _ => 0.0,
            },
            strafe: match instr.strafe {
                -1 => -1.0,
                0 => 0.0,
                1 => 1.0,
                _ => 0.0,
            },
            jump: instr.jump,
            sneak: instr.sneak,
        }
    }
}

pub mod cat;
pub mod components;
pub mod ecs;
pub mod registration;
pub mod systems;

// The player is not here because EntityDef requires us to make a template function but a player
// entity doesn't have a sensible default.
define_entities! {
    CAT => {
        ident: "cat",
        template: crate::entity::cat::cat_template,
    },
}
