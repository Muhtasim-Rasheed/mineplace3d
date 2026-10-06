use std::borrow::Cow;

use crate::{
    block::{BlockId, block_registry},
    direction::Direction,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BlockState {
    pub block: BlockId,
    pub data: u128,
}

impl BlockState {
    pub fn default_for(block: BlockId) -> Self {
        Self {
            block,
            data: block_registry().get(block).unwrap().default_state,
        }
    }

    pub fn get_str(&self, name: &str) -> Option<Cow<'static, str>> {
        let def = block_registry().get(self.block).unwrap();
        let p = def.property(name)?;
        Some((p.value_name)(p.get(self.data)))
    }

    pub fn set_str(&mut self, name: &str, value: &str) -> bool {
        let def = block_registry().get(self.block).unwrap();
        let Some(p) = def.property(name) else {
            return false;
        };
        let Some(v) = (p.parse)(value) else {
            return false;
        };
        p.set(&mut self.data, v);
        true
    }

    pub fn maybe_with_str(mut self, name: &str, value: &str) -> Self {
        self.set_str(name, value);
        self
    }

    pub fn get<T: PropertyValue + 'static>(&self, name: &str) -> Option<T> {
        let def = block_registry().get(self.block).unwrap();
        let p = def.property(name)?;
        let _ = p.check::<T>() || return None;
        Some(T::from_index(p.get(self.data)))
    }

    pub fn set<T: PropertyValue + 'static>(&mut self, name: &str, value: T) -> bool {
        let def = block_registry().get(self.block).unwrap();
        let Some(p) = def.property(name) else {
            return false;
        };
        let _ = p.check::<T>() || return false;
        p.set(&mut self.data, value.to_index());
        true
    }

    pub fn maybe_with<T: PropertyValue + 'static>(mut self, name: &str, value: T) -> Self {
        self.set(name, value);
        self
    }

    pub fn named_props(&self) -> Vec<(&'static str, Cow<'static, str>)> {
        let def = block_registry().get(self.block).unwrap();
        def.state_properties
            .iter()
            .filter_map(|p| {
                let v = p.get(self.data);
                (v != p.default).then(|| (p.name, (p.value_name)(v)))
            })
            .collect()
    }
}

impl std::fmt::Debug for BlockState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}[{}]",
            block_registry().get(self.block).unwrap().ident,
            self.named_props()
                .into_iter()
                .map(|(name, val)| format!("{name}={val}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockStateMatcher {
    pub set_bits: u128,
    pub requirement: u128,
}

impl BlockStateMatcher {
    pub fn new(set_bits: u128, requirement: u128) -> Self {
        Self {
            set_bits,
            requirement: requirement & set_bits,
        }
    }

    pub fn parse(block: BlockId, state_str: &str) -> Result<Self, String> {
        let mut blockstate = BlockState::default_for(block);
        let blockdef = block_registry().get(block).unwrap();
        let mut set_bits = 0u128;

        for part in state_str
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let (name, val) = part
                .split_once('=')
                .ok_or_else(|| format!("expected '=' in property '{part}'"))?;
            let (name, val) = (name.trim(), val.trim());

            if !blockstate.set_str(name, val) {
                return Err(format!("unknown property or invalid value: {name}={val}"));
            }

            let prop = blockdef.property(name).unwrap();
            set_bits |= prop.mask << prop.shift;
        }

        Ok(Self::new(set_bits, blockstate.data))
    }

    #[inline]
    pub fn matches(self, state_data: u128) -> bool {
        state_data & self.set_bits == self.requirement
    }
}

impl std::str::FromStr for BlockState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (ident, props) = match s.trim().split_once('[') {
            Some((ident, rest)) => {
                let props = rest
                    .strip_suffix(']')
                    .ok_or_else(|| "missing closing `]`".to_string())?;
                (ident, Some(props))
            }
            None => (s, None),
        };

        let block = block_registry()
            .get_id(ident)
            .ok_or_else(|| format!("unknown block `{ident}`"))?;

        let mut state = Self::default_for(block);

        let Some(props) = props else {
            return Ok(state);
        };
        if props.is_empty() {
            return Ok(state);
        }
        for prop in props.split(',') {
            let (name, value) = prop
                .split_once('=')
                .ok_or_else(|| format!("invalid property `{prop}`"))?;

            if !state.set_str(name, value) {
                return Err(format!(
                    "invalid property `{name}={value}` for block `{ident}`"
                ));
            }
        }

        Ok(state)
    }
}

pub trait PropertyValue {
    const COUNT: u128;

    fn to_index(self) -> u128;
    fn from_index(i: u128) -> Self;
    fn name(self) -> Cow<'static, str>;
    fn parse(s: &str) -> Option<Self>
    where
        Self: Sized;
}

impl PropertyValue for bool {
    const COUNT: u128 = 2;

    fn to_index(self) -> u128 {
        self as u128
    }

    fn from_index(i: u128) -> Self {
        i != 0
    }

    fn name(self) -> Cow<'static, str> {
        (if self { "true" } else { "false" }).into()
    }

    fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SlabHalf {
    Bottom = 0,
    Top = 1,
    Both = 2,
}

impl PropertyValue for SlabHalf {
    const COUNT: u128 = 3;

    fn to_index(self) -> u128 {
        self as u128
    }

    fn from_index(i: u128) -> Self {
        match i {
            0 => Self::Bottom,
            1 => Self::Top,
            2 => Self::Both,
            _ => unreachable!(),
        }
    }

    fn name(self) -> Cow<'static, str> {
        match self {
            Self::Bottom => "bottom".into(),
            Self::Top => "top".into(),
            Self::Both => "both".into(),
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "bottom" => Some(Self::Bottom),
            "top" => Some(Self::Top),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HorizontalDir(pub Direction);

impl PropertyValue for HorizontalDir {
    const COUNT: u128 = 4;

    fn to_index(self) -> u128 {
        match self.0 {
            Direction::North => 0,
            Direction::South => 1,
            Direction::East => 2,
            Direction::West => 3,
            _ => unreachable!(),
        }
    }

    fn from_index(i: u128) -> Self {
        match i {
            0 => HorizontalDir(Direction::North),
            1 => HorizontalDir(Direction::South),
            2 => HorizontalDir(Direction::East),
            3 => HorizontalDir(Direction::West),
            _ => unreachable!(),
        }
    }

    fn name(self) -> Cow<'static, str> {
        self.0.to_str().into()
    }

    fn parse(s: &str) -> Option<Self> {
        s.parse::<Direction>()
            .ok()
            .filter(|&d| d != Direction::Up && d != Direction::Down)
            .map(Self)
    }
}
