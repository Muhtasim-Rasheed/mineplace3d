//! Implementation of the /clear command

use crate::{
    command::{ArgStream, Command, CommandContext},
    entity::PlayerEntity,
    textcomponent::TextComponent,
};

pub struct ClearCommand;

const DESC: &str = r#"
`clear` - Clears the sender's inventory.

Usage: `/clear`

Example: `/clear` will clear the sender's inventory.
"#;

impl Command for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(&self, ctx: &mut CommandContext, args: ArgStream) -> Result<TextComponent, String> {
        let sender = match ctx.get_sender() {
            Ok(entity) => entity,
            Err(e) => {
                log::error!("{}", e);
                return Err("You must be connected to use this command".to_string());
            }
        };

        args.ensure_empty()?;

        if let Some(player) = sender.as_any_mut().downcast_mut::<PlayerEntity>() {
            player.inventory.clear();

            Ok("%b7FCleared your inventory".parse().unwrap())
        } else {
            Err("You aren't a player".to_string())
        }
    }
}
