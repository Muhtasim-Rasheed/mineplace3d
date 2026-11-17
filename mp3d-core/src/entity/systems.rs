use crate::{
    entity::{
        MoveInput,
        components::{Flying, Hitbox, OnGround, Position, Rotation, Velocity},
        item::PICKUP_RADIUS,
    },
    item::{Inventory, ItemStack},
    physics::{PhysicsState, step},
    world::World,
};

pub fn moving(world: &mut World, dt: f32) {
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
            flying: flying.as_ref().is_some_and(|v| v.0),
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

pub fn picking_up_item(world: &mut World, _dt: f32) {
    let items: Vec<_> = world
        .ecs
        .query::<(&Position, &ItemStack)>()
        .map(|(e, (pos, _item))| (e, pos.0))
        .collect();

    let mut pickups: Vec<_> = Vec::new();
    for (collector, (pos, _inv)) in world.ecs.query::<(&Position, &Inventory)>() {
        for &(item, item_pos) in &items {
            if pos.0.distance_squared(item_pos) <= PICKUP_RADIUS * PICKUP_RADIUS {
                pickups.push((collector, item));
            }
        }
    }

    for (collector, item) in pickups {
        let Some(stack) = world.ecs.get_component_copied::<ItemStack>(item) else {
            // already picked up by someone else
            continue;
        };
        let Some(inv) = world.ecs.get_component_mut::<Inventory>(collector) else {
            // entity randomly despawns I guess?
            continue;
        };
        inv.add_stack_single(stack);
        world.ecs.despawn(item);
    }
}
