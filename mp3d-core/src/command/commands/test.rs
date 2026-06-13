// test.rs

/* This is an implementation of a test command to run differnent kinds of server-side tests on mp3d-core.
 * It currently has 2 test modes: `pass` and `error`
 * The `pass` mode simulates a successful command with coloured text
 * The `error` mode simulates a command error
*/

// import crates
use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext, parser::Word},
    textcomponent::TextComponent,
};
// declare the struct for the test command
pub struct TestCommand;

// description for the command
const DESC: &str = r#"
`test` - Runs a set of server-side tests

Usage: `/test <pass/error>`

Example: `/test error` will run a test of a command error
"#;

// implementation of the command
impl Command for TestCommand {
    // the name
    fn name(&self) -> &'static str {
        "test"
    }

    // description trim
    fn description(&self) -> &'static str {
        DESC.trim()
    }

    // command logic
    fn execute(
        &self,
        ctx: &mut CommandContext,
        mut args: ArgStream,
    ) -> Result<TextComponent, String> {
        // parse the arguments the to command to tell the game the testing mode
        let mode = Word::parse(&mut args)?;
        args.ensure_empty()?;

        // error test mode
        if mode.0 == "error" {
            // throw an error
            return Err(format!(
                "Current seed: {}\nCurrent tps: {}\nTHIS IS A TEST ERROR",
                ctx.world.generator.seed(),
                ctx.tps
            ));
        } else if mode.0 == "pass" {
            // testing for a succesfully run command + colour testing
            return Ok(format!(
                "%xFF0000FF RED %x00FF00FF GREEN %x0000FFFF BLUE %x000000FF BLACK %xFFFFFFFF WHITE \nCurrent seed: {} \nCurrent tps: {}",
                ctx.world.generator.seed(),
                ctx.tps
            )
            .parse()
            .unwrap());
        }
        Err(format!("`{}` is not a valid test mode!", mode.0)) // handle an invalid argument
    }
}
