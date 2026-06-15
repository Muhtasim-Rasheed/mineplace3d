//! Implementation of the /tps command

use crate::{
    command::{ArgStream, Command, CommandContext},
    textcomponent::TextComponent,
};

pub struct TpsCommand;

const DESC: &str = r#"
`tps` - Output current TPS (Ticks Per Second) of the server.

Usage: `/tps`

Example: `/tps` outputs current TPS (usually 48).
"#;

impl Command for TpsCommand {
    fn name(&self) -> &'static str {
        "tps"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(&self, ctx: &mut CommandContext, args: ArgStream) -> Result<TextComponent, String> {
        args.ensure_empty()?;

        Ok(format!("Current TPS: {}%r", ctx.tps).parse().unwrap())
    }
}
