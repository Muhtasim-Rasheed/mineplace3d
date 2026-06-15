//! Implementation of the /test command

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext},
    textcomponent::TextComponent,
};

pub struct TestCommand;

const DESC: &str = r#"
`test` - Runs a set of server-side tests

Usage: `/test <pass | error>`

Example: `/test error` will run a test of a command error
"#;

enum Subcommand {
    Pass,
    Error,
}

impl CommandArg for Subcommand {
    fn parse(args: &mut ArgStream) -> Result<Self, String> {
        match args.next() {
            Some("pass") => Ok(Self::Pass),
            Some("error") => Ok(Self::Error),
            Some(s) => Err(format!("Invalid test mode '{}'", s)),
            None => Err(format!("Expected a test mode but got nothing")),
        }
    }
}

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
        let mode = Subcommand::parse(&mut args)?;
        args.ensure_empty()?;

        match mode {
            Subcommand::Error => Err(format!(
                "Current seed: {}\nCurrent tps: {}\nTHIS IS A TEST ERROR",
                ctx.world.generator.seed(),
                ctx.tps
            )),
            Subcommand::Pass => Ok(format!(
                "%xFF0000FF RED %x00FF00FF GREEN %x0000FFFF BLUE %x000000FF BLACK %xFFFFFFFF WHITE\nCurrent seed: {}\nCurrent tps: {}%r",
                ctx.world.generator.seed(),
                ctx.tps
            )
            .parse()
            .unwrap()),
        }
    }
}
