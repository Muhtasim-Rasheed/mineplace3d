//! Game entities for Mineplace3D.
//!
//! This module provides the `Entity` trait and some implementations like the `Player` entity.

use glam::Vec3;

use crate::{saving::Saveable, world::World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EntityType {
    Player = 0,
}

/// Represents a game entity in the world.
pub trait Entity: std::any::Any + Saveable + Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>().rsplit("::").next().unwrap()
    }
    fn entity_type(&self) -> EntityType;
    fn set_id(&mut self, id: u64);
    fn id(&self) -> u64;
    fn snapshot(&self) -> Vec<u8>;
    fn position(&self) -> Vec3;
    fn apply_velocity(&mut self, velocity: Vec3);
    fn width() -> f32
    where
        Self: Sized;
    fn height() -> f32
    where
        Self: Sized;
    fn requests_removal(&self) -> bool {
        false
    }
    /// Called every 48 ticks per second.
    fn tick(&mut self, world: &mut World, tps: u8);
}

pub mod player;

pub use player::*;
