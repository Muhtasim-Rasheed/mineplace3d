use crate::{
    entity::{
        MoveInput,
        components::{Flying, Hitbox, OnGround, Position, Rotation, Velocity},
    },
    physics::{PhysicsState, step},
    world::World,
};

pub fn movement_system(world: &mut World, dt: f32) {
    for (_, (pos, vel, on_ground, flying, rotation, hitbox, input)) in world.ecs.query::<(
        &mut Position,
        &mut Velocity,
        &mut OnGround,
        Option<&mut Flying>,
        &Rotation,
        &Hitbox,
        Option<&MoveInput>,
    )>() {
        let input = input.copied();
        let state = PhysicsState {
            position: pos.0,
            velocity: vel.0,
            on_ground: on_ground.0,
            flying: flying.as_ref().map_or(false, |v| v.0),
        };
        let state = step(
            state,
            input.unwrap_or_default(),
            rotation.yaw,
            hitbox.width,
            hitbox.height,
            &world.chunks,
            dt,
        );
        *pos = Position(state.position);
        *vel = Velocity(state.velocity);
        *on_ground = OnGround(state.on_ground);
        if let Some(flying) = flying {
            *flying = Flying(state.flying);
        }
    }
}
