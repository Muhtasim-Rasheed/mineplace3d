//! Implementation of the /setblock command

use crate::{
    block::{BlockState, block_registry},
    command::{
        ArgStream, Command, CommandArg, CommandContext,
        parser::{Coord3, Word},
    },
    textcomponent::TextComponent,
};

pub struct SetBlockCommand;

const DESC: &str = r#"
`setblock` - Set a block at the specified coordinates, optionally specifying blockstate aswell.

Usage: `/setblock block_ident x y z [state_data]`
The block identifier is a string that identifies a block. A coordinate can be a number (e.g. "100.5"), be relative from the player's position (e.g. "~4") or scale on the player's forward direction. Finally, the state_data is a 16-bit integer that defines the blocks behavior and appearance.

Example: `/setblock stone_slab ~ ~10 ~ 1` places a top-slab 10 blocks above the player.
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

        let ident = Word::parse(&mut args)?;
        let coord3 = Coord3::parse(&mut args)?;
        let state_data = <Option<u16>>::parse(&mut args)?;
        args.ensure_empty()?;

        let reg = block_registry();
        let block = reg.get_id(&ident.0).ok_or("Unknown block identifier")?;
        let block_def = reg.get(block).unwrap();
        let ivec3 = coord3.as_ivec3(sender.position(), sender.forward());
        let state = if let Some(state_data) = state_data {
            if BlockState::possible_data_values(block_def.state_type)
                .unwrap()
                .contains(&state_data)
            {
                BlockState::new(block_def.state_type, state_data)
            } else {
                return Err("Invalid block state data for this block".to_string());
            }
        } else {
            BlockState::default_state(block_def.state_type).unwrap()
        };

        ctx.world.urgent_set_block_at(
            ivec3,
            block,
            state,
            crate::protocol::BlockUpdateKind::Placed,
        );
        Ok(format!(
            "%b7FSet block at {}, {}, {} to {}%r",
            ivec3.x, ivec3.y, ivec3.z, block_def.ident
        )
        .parse()
        .unwrap())
    }
}
