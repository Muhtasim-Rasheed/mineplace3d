use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext, parser::GreedyString},
    textcomponent::TextComponent,
};

pub struct SayCommand;

const DESC: &str = r#"
`say` - Make the sender say something in the chat.

Usage: `/say word1 word2`

Example: `/say Hello world!` will make the sender say "Hello world!" in the chat.
"#;

impl Command for SayCommand {
    fn name(&self) -> &'static str {
        "say"
    }

    fn description(&self) -> &'static str {
        DESC.trim()
    }

    fn execute(
        &self,
        ctx: &mut CommandContext,
        mut args: ArgStream,
    ) -> Result<TextComponent, String> {
        let sender = match ctx.get_sender_session() {
            Ok(session) => session,
            Err(e) => {
                log::error!("{}", e);
                return Err("You must be connected to use this command".to_string());
            }
        };

        let text = GreedyString::parse(&mut args)?.0;
        args.ensure_empty()?;

        sender.send_chat_message(text.clone())?;
        Ok(format!("%bA9You said: {}%r", text).parse().unwrap())
    }
}
