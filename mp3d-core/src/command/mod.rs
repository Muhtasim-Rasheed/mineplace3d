use fxhash::FxHashMap;

use crate::{entity::Entity, server::PlayerSession, textcomponent::TextComponent, world::World};

pub mod commands;
mod parser;

/// Context passed to command execution, containing mutable access to the server and the connection
/// ID of the command sender.
pub struct CommandContext<'a> {
    pub connections: &'a FxHashMap<u64, u64>,
    pub sessions: &'a mut FxHashMap<u64, PlayerSession>,
    pub world: &'a mut World,
    pub connection_id: u64,
    pub tps: u8,
}

impl<'a> CommandContext<'a> {
    pub fn get_sender_session(&mut self) -> Result<&mut PlayerSession, String> {
        let session_id = *self.connections.get(&self.connection_id).ok_or_else(|| {
            format!(
                "Connection {} doesn't have an associated session id",
                self.connection_id
            )
        })?;
        self.sessions.get_mut(&session_id).ok_or_else(|| {
            format!(
                "Session {} (Connection {}) doesn't exist",
                session_id, self.connection_id,
            )
        })
    }

    pub fn get_sender(&mut self) -> Result<&mut dyn Entity, String> {
        let session_id = *self.connections.get(&self.connection_id).ok_or_else(|| {
            format!(
                "Connection {} doesn't have an associated session id",
                self.connection_id
            )
        })?;
        let entity_id = self
            .sessions
            .get(&session_id)
            .map(|v| v.entity_id)
            .ok_or_else(|| {
                format!(
                    "Session {} (Connection {}) doesn't have an associated entity id",
                    session_id, self.connection_id,
                )
            })?;
        self.world
            .entities
            .get_mut(&entity_id)
            .map(|v| v.as_mut())
            .ok_or_else(|| {
                format!(
                    "Session {} (Connection {}) doesn't have an associated entity",
                    session_id, self.connection_id,
                )
            })
    }
}

/// Manager for registering and executing commands.
pub struct CommandManager {
    commands: FxHashMap<&'static str, Box<dyn Command>>,
}

impl CommandManager {
    pub fn new() -> Self {
        Self {
            commands: FxHashMap::default(),
        }
    }

    /// Registers a command for execution. The command must implement the [`Command`] trait, which
    /// allows it to be executed through dynamic dispatch.
    pub fn register<C: Command + 'static>(&mut self, command: C) {
        self.commands.insert(command.name(), Box::new(command));
    }

    /// Executes a command by name with the given context and arguments. The arguments are passed as
    /// a slice of strings, which the implementation should parse according to the expected argument
    /// types. The implementation can return an optional [`TextComponent`] to send as a response to
    /// the command, or an error message if the execution fails (e.g. due to invalid arguments).
    pub fn execute(
        &mut self,
        ctx: &mut CommandContext,
        args: &[&str],
    ) -> Result<Option<TextComponent>, String> {
        let mut args = ArgStream::new(args);

        if let Some(name) = args.next().and_then(|v| v.strip_prefix('/')) {
            if let Some(command) = self.commands.get(name) {
                command.execute(ctx, args).map(Some)
            } else {
                Err(format!("Unknown command: {}", name))
            }
        } else {
            Ok(None)
        }
    }
}

pub struct ArgStream<'a> {
    iter: std::iter::Peekable<std::slice::Iter<'a, &'a str>>,
}

impl<'a> ArgStream<'a> {
    pub fn new(slice: &'a [&'a str]) -> Self {
        ArgStream {
            iter: slice.iter().peekable(),
        }
    }

    pub fn peek(&mut self) -> Option<&'a str> {
        self.iter.peek().map(|&v| *v)
    }

    pub fn next(&mut self) -> Option<&'a str> {
        self.iter.next().copied()
    }

    pub fn rest(&mut self) -> String {
        let mut string = String::new();
        while let Some(s) = self.next() {
            string.push_str(s);
            string.push(' ');
        }
        string
    }

    pub fn ensure_empty(mut self) -> Result<(), String> {
        if self.peek().is_none() {
            Ok(())
        } else {
            Err(format!("Leftover arguments: {}", self.rest()))
        }
    }
}

/// Trait for parsing a single command argument from a string. This can be implemented for various
/// types.
pub trait CommandArg: Sized {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String>;
}

/// Object-safe version of [`TypedCommand`] for dynamic dispatch. The [`Command::execute`] method takes a slice
/// of strings as arguments and parses them internally.
pub trait Command {
    /// Returns the name of the command, e.g. "tp".
    fn name(&self) -> &'static str;

    /// Returns a short description of the command for help messages.
    fn description(&self) -> &'static str;

    /// Executes the command with the given context and arguments. The arguments are passed as a
    /// slice of strings, which the implementation should parse according to the expected argument
    /// types. The implementation can return an optional [`TextComponent`] to send as a response to
    /// the command, or an error message if the execution fails (e.g. due to invalid arguments).
    fn execute(&self, ctx: &mut CommandContext, args: ArgStream) -> Result<TextComponent, String>;
}
