//! Implementation of the /spawn command

use glam::Vec3;

use crate::{
    command::{
        ArgStream, Command, CommandArg, CommandContext,
        parser::{Coord3, CoordArg, Word},
    },
    entity::{
        components::{Position, Rotation},
        registration::entity_registry,
    },
    protocol::S2CMessage,
    textcomponent::TextComponent,
};

pub struct SpawnCommand;

const DESC: &str = r#"
`spawn` - Summon a new entity into the world.

Usage: `/spawn entity_ident [x y z]`
The entity ident is a string that identifies an entity. A coordinate can be a number (e.g. "100.5"), be relative from the player's position (e.g. "~4") or scale on the player's forward direction (e.g. "^10").

Example: `/spawn cat` spawns a cat at the sender's position.
"#;

impl Command for SpawnCommand {
    fn name(&self) -> &'static str {
        "spawn"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(
        &self,
        ctx: &mut CommandContext,
        mut args: ArgStream,
    ) -> Result<TextComponent, String> {
        let sender = match ctx.get_sender() {
            Ok(entity) => entity,
            Err(e) => {
                log::error!("{}", e);
                return Err("You must be connected to use this command".to_string());
            }
        };

        let Some((Position(pos), rot)) = ctx
            .world
            .ecs
            .get_component_copied::<Position>(sender)
            .and_then(|v| Some((v, ctx.world.ecs.get_component_copied::<Rotation>(sender)?)))
        else {
            return Err("You must have a position and rotation associated with you".to_string());
        };

        let entity_ident = Word::parse(&mut args)?;
        let coord3 = <Option<Coord3>>::parse(&mut args)?;
        args.ensure_empty()?;

        let yaw_rad = rot.yaw.to_radians();
        let pitch_rad = rot.pitch.to_radians();
        let fwd = Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        );

        let pos = coord3
            .unwrap_or(Coord3 {
                x: CoordArg::Relative(0.0),
                y: CoordArg::Relative(0.0),
                z: CoordArg::Relative(0.0),
            })
            .as_vec3(pos, fwd);

        let reg = entity_registry();
        let entity_def_id = reg
            .get_id(&entity_ident.0)
            .ok_or("Unknown entity identifier")?;
        let entity_id = ctx.world.ecs.spawn_entity(entity_def_id, pos);

        let entity_details = ctx.world.ecs.serialize_entity(entity_id);
        ctx.get_sender_session()
            .unwrap()
            .pending_messages
            .push(S2CMessage::EntitySpawned {
                entity_id,
                entity_details,
            });

        Ok(format!(
            "%b7FSpawned {} entity at {}, {}, {} with ID {entity_id}%r",
            entity_ident.0, pos.x, pos.y, pos.z
        )
        .parse()
        .unwrap())
    }
}
