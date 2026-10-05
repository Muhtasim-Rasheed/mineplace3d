//! Implementation of the /setblock command

use glam::Vec3;

use crate::{
    command::{
        ArgStream, Command, CommandArg, CommandContext,
        parser::{Coord3, GreedyString},
    },
    entity::components::{Position, Rotation},
    textcomponent::TextComponent,
};

pub struct SetBlockCommand;

const DESC: &str = r#"
`setblock` - Set a block at the specified coordinates, optionally specifying blockstate aswell.

Usage: `/setblock x y z block_ident[state_property=state_value...]`
The block identifier is a string that identifies a block. A coordinate can be a number (e.g. "100.5"), be relative from the player's position (e.g. "~4") or scale on the player's forward direction (e.g. "^10"). Finally, the state_data is a 16-bit integer that defines the blocks behavior and appearance.

Example: `/setblock ~ ~10 ~ stone_slab[half=top]` places a top-slab 10 blocks above the player.
"#;

impl Command for SetBlockCommand {
    fn name(&self) -> &'static str {
        "setblock"
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

        let yaw_rad = rot.yaw.to_radians();
        let pitch_rad = rot.pitch.to_radians();
        let fwd = Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        );

        let coord3 = Coord3::parse(&mut args)?;
        let state_str = GreedyString::parse(&mut args)?;
        args.ensure_empty()?;

        let state = match state_str.0.parse() {
            Ok(s) => s,
            Err(e) => return Err(format!("Invalid blockstate: {e}")),
        };
        let ivec3 = coord3.as_ivec3(pos, fwd);

        ctx.world
            .urgent_set_block_at(ivec3, state, crate::protocol::BlockUpdateKind::Placed);
        Ok(format!(
            "%b7FSet block at {}, {}, {} to {}%r",
            ivec3.x, ivec3.y, ivec3.z, state_str.0
        )
        .parse()
        .unwrap())
    }
}
