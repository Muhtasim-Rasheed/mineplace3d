use glam::{IVec3, Vec3};

use crate::command::{ArgStream, CommandArg};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordArg {
    /// Absolute coordinate, e.g. "100" or "100.5".
    Absolute(f32),
    /// Relative coordinate, e.g. "~", "~1", or "~1.5".
    Relative(f32),
    /// Forward-relative coordinate, e.g. "^", "^1", or "^1.5".
    ForwardRelative(f32),
}

impl CoordArg {
    pub fn as_f32(self, axis_pos: f32, axis_forward: f32) -> f32 {
        match self {
            Self::Absolute(a) => a,
            Self::Relative(r) => axis_pos + r,
            Self::ForwardRelative(fr) => axis_forward * fr + axis_pos,
        }
    }
}

impl CommandArg for CoordArg {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String> {
        let arg = args.next().ok_or("Expected a coordinate but got nothing")?;

        if let Some(stripped) = arg.strip_prefix("~") {
            if stripped.is_empty() {
                Ok(Self::Relative(0.0))
            } else {
                Ok(Self::Relative(
                    stripped
                        .parse()
                        .map_err(|_| format!("Invalid coordinate: {}", arg))?,
                ))
            }
        } else if let Some(stripped) = arg.strip_prefix("^") {
            if stripped.is_empty() {
                Ok(Self::ForwardRelative(0.0))
            } else {
                Ok(Self::ForwardRelative(
                    stripped
                        .parse()
                        .map_err(|_| format!("Invalid coordinate: {}", arg))?,
                ))
            }
        } else {
            Ok(Self::Absolute(
                arg.parse()
                    .map_err(|_| format!("Invalid coordinate: {}", arg))?,
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord3 {
    x: CoordArg,
    y: CoordArg,
    z: CoordArg,
}

impl Coord3 {
    pub fn as_vec3(self, pos: Vec3, forward: Vec3) -> Vec3 {
        Vec3::new(
            self.x.as_f32(pos.x, forward.x),
            self.y.as_f32(pos.y, forward.y),
            self.z.as_f32(pos.z, forward.z),
        )
    }

    pub fn as_ivec3(self, pos: Vec3, forward: Vec3) -> IVec3 {
        (self.as_vec3(pos, forward) - Vec3::new(0.5, 0.0, 0.5)).as_ivec3()
    }
}

impl CommandArg for Coord3 {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String> {
        Ok(Self {
            x: CoordArg::parse(args)?,
            y: CoordArg::parse(args)?,
            z: CoordArg::parse(args)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreedyString(pub String);

impl CommandArg for Word {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String> {
        let arg = args.next().ok_or("Expected a string but got nothing")?;
        Ok(Word(arg.to_string()))
    }
}

impl CommandArg for GreedyString {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String> {
        Ok(GreedyString(args.rest()))
    }
}

impl CommandArg for u16 {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String> {
        let arg = args
            .next()
            .ok_or("Expected a number (16-bit integer) but got nothing")?;
        arg.parse()
            .map_err(|e| format!("Invalid integer '{}': {}", arg, e))
    }
}

impl<A: CommandArg> CommandArg for Option<A> {
    fn parse<'a>(args: &mut ArgStream) -> Result<Self, String> {
        if args.peek().is_some() {
            A::parse(args).map(Some)
        } else {
            Ok(None)
        }
    }
}
