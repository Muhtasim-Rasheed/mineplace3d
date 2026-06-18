//! Implementation of the /give command

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext, parser::Word},
    entity::PlayerEntity,
    item::item_registry,
    textcomponent::TextComponent,
};

pub struct GiveCommand;

const DESC: &str = r#"
`give` - Gives an item the specified amount of times to the sender.

Usage: `/give item_ident [count]`
The item identifier a string that identifies an item. The count is optional and defaults to 1.

Example: `/give grass_block 10` will give the sender 10 grass blocks.
"#;

impl Command for GiveCommand {
    fn name(&self) -> &'static str {
        "give"
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
        let count = <Option<u16>>::parse(&mut args)?.unwrap_or(1);
        args.ensure_empty()?;

        let reg = item_registry();
        let item = reg.get_id(&ident.0).ok_or("Unknown item identifier")?;
        let item_def = reg.get(item).unwrap();
        if let Some(player) = sender.as_any_mut().downcast_mut::<PlayerEntity>() {
            player.inventory.add_stack(item, count);

            Ok(format!("%b7FGave you {} x {}%r", count, item_def.ident)
                .parse()
                .unwrap())
        } else {
            Err("You aren't a player".to_string())
        }
    }
}
