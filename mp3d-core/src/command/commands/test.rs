//! Implementation of the /test command

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext, parser::Word},
    textcomponent::TextComponent,
};

pub struct TestCommand;

const DESC: &str = r#"
`test` - Runs a set of server-side tests

Usage: `/test <pass | error>`

Example: `/test error` will run a test of a command error
"#;

impl Command for TestCommand {
    fn name(&self) -> &'static str {
        "test"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(
        &self,
        ctx: &mut CommandContext,
        mut args: ArgStream,
    ) -> Result<TextComponent, String> {
        let mode = Word::parse(&mut args)?;
        args.ensure_empty()?;

        if mode.0 == "error" {
            return Err(format!(
                "Current seed: {}\nCurrent tps: {}\nTHIS IS A TEST ERROR",
                ctx.world.generator.seed(),
                ctx.tps
            ));
        } else if mode.0 == "pass" {
            return Ok(format!(
                "%xFF0000FF RED %x00FF00FF GREEN %x0000FFFF BLUE %x000000FF BLACK %xFFFFFFFF WHITE\nCurrent seed: {}\nCurrent tps: {}",
                ctx.world.generator.seed(),
                ctx.tps
            )
            .parse()
            .unwrap());
        }
        Err(format!("`{}` is not a valid test mode!", mode.0))
    }
}
