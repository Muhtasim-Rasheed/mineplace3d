//! Minimal standard binary NBT reader/writer for world saves.
use super::WorldLoadError;
use std::io::{Cursor, Read};
const END: u8 = 0;
const BYTE: u8 = 1;
const INT: u8 = 3;
const LONG: u8 = 4;
const BYTE_ARRAY: u8 = 7;
const STRING: u8 = 8;
const LIST: u8 = 9;
const COMPOUND: u8 = 10;
const MAX_DEPTH: usize = 64;
#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    Byte(i8),
    Int(i32),
    Long(i64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<Tag>),
    Compound(Vec<(String, Tag)>),
}
impl Tag {
    pub fn compound(entries: impl IntoIterator<Item = (impl Into<String>, Tag)>) -> Self {
        Self::Compound(entries.into_iter().map(|(n, t)| (n.into(), t)).collect())
    }
    pub fn list(values: impl IntoIterator<Item = Tag>) -> Self {
        Self::List(values.into_iter().collect())
    }
    pub fn get(&self, name: &str) -> Option<&Tag> {
        let Self::Compound(values) = self else {
            return None;
        };
        values.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
    pub fn byte(&self, name: &str) -> Result<i8, WorldLoadError> {
        match self.get(name) {
            Some(Self::Byte(v)) => Ok(*v),
            _ => Err(field(name, "byte")),
        }
    }
    pub fn int(&self, name: &str) -> Result<i32, WorldLoadError> {
        match self.get(name) {
            Some(Self::Int(v)) => Ok(*v),
            _ => Err(field(name, "int")),
        }
    }
    pub fn long(&self, name: &str) -> Result<i64, WorldLoadError> {
        match self.get(name) {
            Some(Self::Long(v)) => Ok(*v),
            _ => Err(field(name, "long")),
        }
    }
    pub fn string(&self, name: &str) -> Result<&str, WorldLoadError> {
        match self.get(name) {
            Some(Self::String(v)) => Ok(v),
            _ => Err(field(name, "string")),
        }
    }
    pub fn bytes(&self, name: &str) -> Result<&[u8], WorldLoadError> {
        match self.get(name) {
            Some(Self::ByteArray(v)) => Ok(v),
            _ => Err(field(name, "byte array")),
        }
    }
    pub fn list_value(&self, name: &str) -> Result<&[Tag], WorldLoadError> {
        match self.get(name) {
            Some(Self::List(v)) => Ok(v),
            _ => Err(field(name, "list")),
        }
    }
    pub fn compound_value(&self, name: &str) -> Result<&Tag, WorldLoadError> {
        match self.get(name) {
            Some(v @ Self::Compound(_)) => Ok(v),
            _ => Err(field(name, "compound")),
        }
    }
}
fn field(name: &str, expected: &str) -> WorldLoadError {
    WorldLoadError::InvalidSaveFormat(format!(
        "NBT field '{name}' is missing or is not a {expected}"
    ))
}
pub fn encode(root: &Tag) -> Result<Vec<u8>, WorldLoadError> {
    if !matches!(root, Tag::Compound(_)) {
        return Err(WorldLoadError::InvalidSaveFormat(
            "NBT root must be a compound".into(),
        ));
    }
    let mut out = Vec::new();
    named(&mut out, "", root)?;
    Ok(out)
}
pub fn decode(data: &[u8]) -> Result<Tag, WorldLoadError> {
    let mut input = Cursor::new(data);
    let (name, root) = read_named(&mut input, 0)?;
    if !name.is_empty() || !matches!(root, Tag::Compound(_)) {
        return Err(WorldLoadError::InvalidSaveFormat(
            "NBT root must be an unnamed compound".into(),
        ));
    }
    if input.position() != data.len() as u64 {
        return Err(WorldLoadError::InvalidSaveFormat(
            "trailing bytes after NBT root".into(),
        ));
    }
    Ok(root)
}
fn id(tag: &Tag) -> u8 {
    match tag {
        Tag::Byte(_) => BYTE,
        Tag::Int(_) => INT,
        Tag::Long(_) => LONG,
        Tag::ByteArray(_) => BYTE_ARRAY,
        Tag::String(_) => STRING,
        Tag::List(_) => LIST,
        Tag::Compound(_) => COMPOUND,
    }
}
fn named(out: &mut Vec<u8>, name: &str, tag: &Tag) -> Result<(), WorldLoadError> {
    out.push(id(tag));
    string(out, name)?;
    payload(out, tag)
}
fn payload(out: &mut Vec<u8>, tag: &Tag) -> Result<(), WorldLoadError> {
    match tag {
        Tag::Byte(v) => out.push(*v as u8),
        Tag::Int(v) => out.extend(v.to_be_bytes()),
        Tag::Long(v) => out.extend(v.to_be_bytes()),
        Tag::ByteArray(v) => {
            len(out, v.len())?;
            out.extend(v)
        }
        Tag::String(v) => string(out, v)?,
        Tag::List(v) => {
            let typ = v.first().map(id).unwrap_or(END);
            if v.iter().any(|x| id(x) != typ) {
                return Err(WorldLoadError::InvalidSaveFormat(
                    "NBT lists must contain one tag type".into(),
                ));
            }
            out.push(typ);
            len(out, v.len())?;
            for x in v {
                payload(out, x)?
            }
        }
        Tag::Compound(v) => {
            for (n, x) in v {
                named(out, n, x)?
            }
            out.push(END)
        }
    }
    Ok(())
}
fn string(out: &mut Vec<u8>, v: &str) -> Result<(), WorldLoadError> {
    let n = u16::try_from(v.len())
        .map_err(|_| WorldLoadError::InvalidSaveFormat("NBT string exceeds u16 length".into()))?;
    out.extend(n.to_be_bytes());
    out.extend(v.as_bytes());
    Ok(())
}
fn len(out: &mut Vec<u8>, v: usize) -> Result<(), WorldLoadError> {
    out.extend(
        i32::try_from(v)
            .map_err(|_| {
                WorldLoadError::InvalidSaveFormat("NBT collection exceeds i32 length".into())
            })?
            .to_be_bytes(),
    );
    Ok(())
}
fn read_named(input: &mut Cursor<&[u8]>, depth: usize) -> Result<(String, Tag), WorldLoadError> {
    let typ = u8(input)?;
    if typ == END {
        return Err(WorldLoadError::InvalidSaveFormat(
            "unexpected NBT end tag".into(),
        ));
    }
    let name = read_string(input)?;
    Ok((name, read_payload(input, typ, depth)?))
}
fn read_payload(input: &mut Cursor<&[u8]>, typ: u8, depth: usize) -> Result<Tag, WorldLoadError> {
    if depth > MAX_DEPTH {
        return Err(WorldLoadError::InvalidSaveFormat(
            "NBT nesting exceeds 64 levels".into(),
        ));
    }
    Ok(match typ {
        BYTE => Tag::Byte(u8(input)? as i8),
        INT => Tag::Int(i32::from_be_bytes(exact(input)?)),
        LONG => Tag::Long(i64::from_be_bytes(exact(input)?)),
        BYTE_ARRAY => {
            let n = read_len(input)?;
            Tag::ByteArray(bytes(input, n)?)
        }
        STRING => Tag::String(read_string(input)?),
        LIST => {
            let e = u8(input)?;
            let n = read_len(input)?;
            if e == END && n != 0 {
                return Err(WorldLoadError::InvalidSaveFormat(
                    "NBT end-tag list is non-empty".into(),
                ));
            }
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                v.push(read_payload(input, e, depth + 1)?)
            }
            Tag::List(v)
        }
        COMPOUND => {
            let mut v = Vec::new();
            loop {
                let field_type = u8(input)?;
                if field_type == END {
                    break;
                }
                let name = read_string(input)?;
                v.push((name, read_payload(input, field_type, depth + 1)?))
            }
            Tag::Compound(v)
        }
        _ => {
            return Err(WorldLoadError::InvalidSaveFormat(format!(
                "unsupported NBT tag type {typ}"
            )));
        }
    })
}
fn u8(input: &mut Cursor<&[u8]>) -> Result<u8, WorldLoadError> {
    Ok(exact::<1>(input)?[0])
}
fn exact<const N: usize>(input: &mut Cursor<&[u8]>) -> Result<[u8; N], WorldLoadError> {
    let mut v = [0; N];
    input
        .read_exact(&mut v)
        .map_err(|_| WorldLoadError::InvalidSaveFormat("unexpected end of NBT data".into()))?;
    Ok(v)
}
fn read_string(input: &mut Cursor<&[u8]>) -> Result<String, WorldLoadError> {
    let n = u16::from_be_bytes(exact(input)?) as usize;
    String::from_utf8(bytes(input, n)?)
        .map_err(|_| WorldLoadError::InvalidSaveFormat("NBT string is not valid UTF-8".into()))
}
fn read_len(input: &mut Cursor<&[u8]>) -> Result<usize, WorldLoadError> {
    usize::try_from(i32::from_be_bytes(exact(input)?)).map_err(|_| {
        WorldLoadError::InvalidSaveFormat("NBT collection has a negative length".into())
    })
}
fn bytes(input: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<u8>, WorldLoadError> {
    let mut v = vec![0; n];
    input
        .read_exact(&mut v)
        .map_err(|_| WorldLoadError::InvalidSaveFormat("unexpected end of NBT data".into()))?;
    Ok(v)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        let x = Tag::compound([
            ("version", Tag::Int(1)),
            ("items", Tag::list([Tag::String("a".into())])),
        ]);
        assert_eq!(decode(&encode(&x).unwrap()).unwrap(), x)
    }
}
