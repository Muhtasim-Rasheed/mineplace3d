//! Implementation of the /lookat command

use glam::Vec3;

use crate::{
    command::{ArgStream, Command, CommandArg, CommandContext, parser::Coord3},
    entity::components::{Position, Rotation},
    textcomponent::TextComponent,
};

pub struct LookAtCommand;

const DESC: &str = r#"
`lookat` - Set your direction.

Usage: `/lookat <subcommand> <args...>`
  - `/lookat pos <x y z>` Look at a given world position.
  - `/lookat dir <yaw pitch>` Look at a given direction. Angle is given in degrees.

Example: `/lookat dir 0 0` makes the sender look south.
"#;

enum Subcommand {
    Pos(Coord3),
    Dir(f32, f32),
}

impl CommandArg for Subcommand {
    fn parse(args: &mut ArgStream) -> Result<Self, String> {
        match args.next() {
            Some("pos") => Coord3::parse(args).map(Self::Pos),
            Some("dir") => Ok(Self::Dir(f32::parse(args)?, f32::parse(args)?)),
            Some(s) => Err(format!("Unknown subcommand '{}'", s)),
            None => Err(format!("Expected a subcommand but got nothing")),
        }
    }
}

impl Command for LookAtCommand {
    fn name(&self) -> &'static str {
        "lookat"
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

        let subcommand = Subcommand::parse(&mut args)?;
        args.ensure_empty()?;

        let (yaw, pitch) = match subcommand {
            Subcommand::Pos(coord3) => {
                let Some((Position(pos), rot)) = ctx
                    .world
                    .ecs
                    .get_component_copied::<Position>(sender)
                    .and_then(|v| {
                        Some((v, ctx.world.ecs.get_component_copied::<Rotation>(sender)?))
                    })
                else {
                    return Err(
                        "You must have a position and rotation associated with you".to_string()
                    );
                };
                let yaw_rad = rot.yaw.to_radians();
                let pitch_rad = rot.pitch.to_radians();
                let fwd = Vec3::new(
                    yaw_rad.sin() * pitch_rad.cos(),
                    pitch_rad.sin(),
                    yaw_rad.cos() * pitch_rad.cos(),
                );
                let target = coord3.as_vec3(pos, fwd);
                let delta = target - pos;
                let yaw_rad = delta.x.atan2(delta.z);
                let pitch_rad = (delta.y).atan2((delta.x * delta.x + delta.z * delta.z).sqrt());
                let yaw = yaw_rad.to_degrees();
                let pitch = pitch_rad.to_degrees();
                *ctx.world.ecs.get_component_mut::<Rotation>(sender).unwrap() =
                    Rotation { yaw, pitch };
                (yaw, pitch)
            }
            Subcommand::Dir(yaw, pitch) => {
                let yaw = yaw.rem_euclid(360.0);
                let pitch = pitch.clamp(-89.9, 89.9);
                *ctx.world.ecs.get_component_mut::<Rotation>(sender).unwrap() =
                    Rotation { yaw, pitch };
                (yaw, pitch)
            }
        };

        Ok(format!("%b7FSet your yaw and pitch to {yaw} and {pitch}%r")
            .parse()
            .unwrap())
    }
}
