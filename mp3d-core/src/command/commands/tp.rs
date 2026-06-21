//! Implementation of the /tp command

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext, parser::Coord3},
    textcomponent::TextComponent,
};

pub struct TpCommand;

const DESC: &str = r#"
`tp` - Teleports the sender to the specified coordinates.

Usage: `/tp x y z`
A coordinate can be a number (e.g. "100.5"), be relative from the player's position (e.g. "~4") or scale on the player's forward direction (e.g. "^10").

Example: `/tp ~ ~10 ~` moves the player 10 blocks up.
"#;

impl Command for TpCommand {
    fn name(&self) -> &'static str {
        "tp"
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

        let coord3 = Coord3::parse(&mut args)?;
        args.ensure_empty()?;

        let pos = sender.position();
        let fwd = sender.forward();
        let vec3 = coord3.as_vec3(pos, fwd);
        *sender.position_mut() = vec3;
        ctx.world.load_around(pos.as_ivec3());

        Ok(
            format!("%b7FTeleported you to {}, {}, {}%r", vec3.x, vec3.y, vec3.z,)
                .parse()
                .unwrap(),
        )
    }
}
