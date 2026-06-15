//! Implementation of the /help command

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext},
    textcomponent::TextComponent,
};

pub struct HelpCommand;

const DESC: &str = r#"
`help` - Output all commands in a page format without descriptions, or output the description of a specific command.

Usage: `/help <page | command>`
If no page is specified, it defaults to page 1. Each page shows 20 commands.

Example: `/help 2` outputs the second page of the command list, and `/help tp` outputs the description of the `tp` command.
"#;

enum PageOrCommand {
    Page(usize),
    Command(String),
}

impl CommandArg for PageOrCommand {
    fn parse(args: &mut ArgStream) -> Result<Self, String> {
        if let Some(arg) = args.next() {
            if let Ok(page) = arg.parse::<usize>() {
                Ok(PageOrCommand::Page(page))
            } else {
                Ok(PageOrCommand::Command(arg.to_string()))
            }
        } else {
            Ok(PageOrCommand::Page(1))
        }
    }
}

impl Command for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(
        &self,
        ctx: &mut CommandContext,
        mut args: ArgStream,
    ) -> Result<TextComponent, String> {
        let arg = PageOrCommand::parse(&mut args)?;
        args.ensure_empty()?;

        match arg {
            PageOrCommand::Page(page) => {
                let commands = ctx.command_manager.iter().collect::<Vec<_>>();
                let total_pages = commands.len().div_ceil(20);
                if page == 0 || page > total_pages {
                    return Err(format!(
                        "Invalid page number. There are {} pages.",
                        total_pages
                    ));
                }

                let start = (page - 1) * 20;
                let end = usize::min(start + 20, commands.len());
                let list = commands[start..end]
                    .iter()
                    .map(|cmd| format!("%b7F/{}%r", cmd.name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                Ok(
                    format!("Commands (Page {}/{}): {}%r", page, total_pages, list)
                        .parse()
                        .unwrap(),
                )
            }
            PageOrCommand::Command(name) => {
                if let Some(cmd) = ctx.command_manager.get(&name) {
                    Ok(format!("%b7F/{}%bF3\n{}%r", cmd.name(), cmd.description())
                        .parse()
                        .unwrap())
                } else {
                    Err(format!("Unknown command: {}", name))
                }
            }
        }
    }
}
