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
    fn position_mut(&mut self) -> &mut Vec3;
    fn forward(&self) -> Vec3;
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

pub mod player;

pub use player::*;
