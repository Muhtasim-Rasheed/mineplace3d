//! Implementation of the /seed command

use crate::{
    command::{ArgStream, Command, CommandContext},
    textcomponent::TextComponent,
};

pub struct SeedCommand;

const DESC: &str = r#"
`seed` - Output the world seed.

Usage: `/seed`

Example: `/seed` outputs the world seed.
"#;

impl Command for SeedCommand {
    fn name(&self) -> &'static str {
        "seed"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(&self, ctx: &mut CommandContext, args: ArgStream) -> Result<TextComponent, String> {
        args.ensure_empty()?;

        Ok(format!("Current Seed: {}%r", ctx.world.generator.seed())
            .parse()
            .unwrap())
    }
}
