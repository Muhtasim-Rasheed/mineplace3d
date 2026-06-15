//! Implementation of the /time command

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext},
    textcomponent::TextComponent,
};

pub struct TimeCommand;

const DESC: &str = r#"
`time` - Output or modify current time.
There is no day / night system yet, so this command really doesn't do anything other than keep track of time.

Usage: `/time [<get | add | sub>]`
  - `/time get` Output current time.
  - `/time add inc` Increment time by `inc`.
  - `/time sub dec` Decrement time by `dec`.
  - `/time` Default to `/time get`.

Example: `/time` outputs current time.
"#;

enum Subcommand {
    Get,
    Add(u64),
    Sub(u64),
}

impl CommandArg for Subcommand {
    fn parse(args: &mut ArgStream) -> Result<Self, String> {
        match args.next() {
            Some("get") => Ok(Self::Get),
            Some("add") => Ok(Self::Add(u64::parse(args)?)),
            Some("sub") => Ok(Self::Sub(u64::parse(args)?)),
            Some(sub) => Err(format!("Unknown subcommand for time: '{}'", sub)),
            None => Ok(Self::Get),
        }
    }
}

impl Command for TimeCommand {
    fn name(&self) -> &'static str {
        "time"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(
        &self,
        ctx: &mut CommandContext,
        mut args: ArgStream,
    ) -> Result<TextComponent, String> {
        let sub = Subcommand::parse(&mut args)?;
        args.ensure_empty()?;

        match sub {
            Subcommand::Get => Ok(format!("Current time: {}%r", ctx.world.time)
                .parse()
                .unwrap()),
            Subcommand::Add(inc) => match ctx.world.time.checked_add(inc) {
                Some(new) => {
                    ctx.world.time = new;
                    Ok(
                        format!("Added {} to current time and now it's {}.%r", inc, new)
                            .parse()
                            .unwrap(),
                    )
                }
                None => Err(format!(
                    "Adding {} to the current time would cause the time to overflow.%r",
                    inc
                )),
            },
            Subcommand::Sub(dec) => match ctx.world.time.checked_sub(dec) {
                Some(new) => {
                    ctx.world.time = new;
                    Ok(format!(
                        "Subtracted {} from current time and now it's {}.%r",
                        dec, new
                    )
                    .parse()
                    .unwrap())
                }
                None => Err(format!(
                    "Subtracting {} from the current time would cause the time to underflow.%r",
                    dec
                )),
            },
        }
    }
}
