//! Physics implementation for entities in Mineplace3D.

use glam::Vec3;

use crate::{axis::Axis, entity::MoveInput};

pub const GRAVITY: f32 = 32.0;
pub const MAX_FALL_SPEED: f32 = 10000.0;
pub const JUMP_VELOCITY: f32 = 9.0;
pub const STEP_HEIGHT: f32 = 0.6;
pub const WALK_SPEED: f32 = 4.3;
pub const FLY_SPEED: f32 = 8.0;
pub const GROUND_ACCEL: f32 = 14.0;
pub const AIR_ACCEL: f32 = 4.0;
pub const FLY_ACCEL: f32 = 10.0;
const SWEEP_ITERATIONS: u32 = 16;

pub trait CollisionWorld {
    /// Checks for collisions between an entity (using its position, width, and height) and the
    /// blocks in the world. This is used for player movement and other entity interactions with
    /// the world.
    fn collides(&self, pos: Vec3, width: f32, height: f32) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicsState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub flying: bool,
}

pub fn step(
    mut state: PhysicsState,
    input: MoveInput,
    yaw: f32,
    width: f32,
    height: f32,
    world: &impl CollisionWorld,
    dt: f32,
) -> PhysicsState {
    let yaw_rad = yaw.to_radians();
    let forward_vec = Vec3::new(yaw_rad.sin(), 0.0, yaw_rad.cos());
    let right_vec = Vec3::new(yaw_rad.cos(), 0.0, -yaw_rad.sin());

    let target_horizontal = (forward_vec * input.forward + right_vec * input.strafe) * WALK_SPEED;

    let accel = if state.flying {
        FLY_ACCEL
    } else if state.on_ground {
        GROUND_ACCEL
    } else {
        AIR_ACCEL
    };
    let t = 1.0 - (-accel * dt).exp();

    state.velocity.x += (target_horizontal.x - state.velocity.x) * t;
    state.velocity.z += (target_horizontal.z - state.velocity.z) * t;

    if state.flying {
        let target_y = if input.jump {
            FLY_SPEED
        } else if input.sneak {
            -FLY_SPEED
        } else {
            0.0
        };
        state.velocity.y += (target_y - state.velocity.y) * t;
    } else {
        if input.jump && state.on_ground {
            state.velocity.y = JUMP_VELOCITY;
            state.on_ground = false;
        }
        state.velocity.y -= GRAVITY * dt;
        state.velocity.y = state.velocity.y.max(-MAX_FALL_SPEED);
    }

    move_and_collide(state, width, height, world, dt)
}

fn move_and_collide(
    mut state: PhysicsState,
    w: f32,
    h: f32,
    world: &impl CollisionWorld,
    dt: f32,
) -> PhysicsState {
    // X axis
    let (after_x, hit_x) = sweep_axis(
        world,
        state.position,
        state.velocity.x * dt,
        w,
        h,
        |p, d| p.with_x(p.x + d),
    );
    if hit_x && !state.flying {
        if let Some(stepped) = try_step(
            state.position,
            world,
            state.velocity.x * dt,
            w,
            h,
            state.on_ground,
            Axis::X,
        ) {
            state.position = stepped;
        } else {
            state.position = after_x;
            state.velocity.x = 0.0;
        }
    } else {
        state.position = after_x;
        if hit_x {
            state.velocity.x = 0.0;
        }
    }

    // Y axis
    let (after_y, hit_y) = sweep_axis(
        world,
        state.position,
        state.velocity.y * dt,
        w,
        h,
        |p, d| p.with_y(p.y + d),
    );
    state.position = after_y;
    if hit_y {
        if state.velocity.y < 0.0 {
            state.on_ground = true;
        }
        state.velocity.y = 0.0;
    } else if state.velocity.y < 0.0 {
        state.on_ground = false;
    }

    // Z axis
    let (after_z, hit_z) = sweep_axis(
        world,
        state.position,
        state.velocity.z * dt,
        w,
        h,
        |p, d| p.with_z(p.z + d),
    );
    if hit_z && !state.flying {
        if let Some(stepped) = try_step(
            state.position,
            world,
            state.velocity.z * dt,
            w,
            h,
            state.on_ground,
            Axis::Z,
        ) {
            state.position = stepped;
        } else {
            state.position = after_z;
            state.velocity.z = 0.0;
        }
    } else {
        state.position = after_z;
        if hit_z {
            state.velocity.z = 0.0;
        }
    }

    if state.velocity.length_squared() > 10000.0 {
        log::warn!("High velocity: {}", state.velocity);
    }

    state
}

fn try_step(
    position: Vec3,
    world: &impl CollisionWorld,
    delta: f32,
    w: f32,
    h: f32,
    on_ground: bool,
    axis: Axis,
) -> Option<Vec3> {
    if !on_ground {
        return None;
    }

    let (lifted, _) = sweep_axis(world, position, STEP_HEIGHT, w, h, |p, d| p.with_y(p.y + d));

    let (moved, hit) = match axis {
        Axis::X => sweep_axis(world, lifted, delta, w, h, |p, d| p.with_x(p.x + d)),
        Axis::Y => unreachable!(),
        Axis::Z => sweep_axis(world, lifted, delta, w, h, |p, d| p.with_z(p.z + d)),
    };

    if hit { None } else { Some(moved) }
}

fn sweep_axis(
    world: &impl CollisionWorld,
    pos: Vec3,
    delta: f32,
    width: f32,
    height: f32,
    with_axis: impl Fn(Vec3, f32) -> Vec3,
) -> (Vec3, bool) {
    if delta.abs() < f32::EPSILON {
        return (pos, false);
    }
    let target = with_axis(pos, delta);
    if !world.collides(target, width, height) {
        return (target, false);
    }
    let mut safe = 0.0_f32;
    let mut unzafe = delta; // the art of avoiding keywords
    for _ in 0..SWEEP_ITERATIONS {
        let mid = (safe + unzafe) * 0.5;
        if world.collides(with_axis(pos, mid), width, height) {
            unzafe = mid;
        } else {
            safe = mid;
        }
    }
    (with_axis(pos, safe), true)
}
