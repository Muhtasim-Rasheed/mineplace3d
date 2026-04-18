//! The core of the Mineplace3D engine. This module contains essential
//! functionalities and definitions required for the engine to operate
//! such as world management, entity handling, etc.

use glam::{IVec3, Vec3, Vec4};

pub mod block;
pub mod datapack;
pub mod entity;
pub mod item;
pub mod protocol;
pub mod saving;
pub mod server;
pub mod world;

#[derive(Debug, Clone, PartialEq)]
pub struct TextComponent {
    pub parts: Vec<TextComponentPart>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextComponentColor {
    Basic(u8),
    Hex(Vec4),
    None,
}

impl From<TextComponentColor> for Vec4 {
    fn from(color: TextComponentColor) -> Self {
        match color {
            TextComponentColor::Basic(code) => {
                let r = (code & 0xC0) >> 6;
                let g = (code & 0x30) >> 4;
                let b = (code & 0x0C) >> 2;
                let a = code & 0x03;
                Vec4::new(
                    r as f32 / 3.0,
                    g as f32 / 3.0,
                    b as f32 / 3.0,
                    (a as f32 + 1.0) / 4.0,
                )
            }
            TextComponentColor::Hex(rgba) => rgba,
            TextComponentColor::None => Vec4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextComponentPart {
    pub text: String,
    pub color: TextComponentColor,
}

impl std::str::FromStr for TextComponent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = Vec::new();
        let mut chars = s.chars().peekable();
        let mut current_text = String::new();
        let mut current_color = TextComponentColor::None;
        while let Some(c) = chars.next() {
            if c == '%' {
                if !current_text.is_empty() {
                    parts.push(TextComponentPart {
                        text: current_text.clone(),
                        color: current_color,
                    });
                    current_text.clear();
                }
                match chars.next() {
                    // Set basic color
                    Some('b') => {
                        // we require exactly 2 hex digits for the basic color code
                        let mut color_str = String::new();
                        for _ in 0..2 {
                            match chars.next() {
                                Some(c) if c.is_ascii_hexdigit() => color_str.push(c),
                                _ => return Err("Invalid basic color code".to_string()),
                            }
                        }
                        if color_str.len() != 2 {
                            return Err(format!("Invalid basic color code: {}", color_str));
                        }
                        let color_value = u8::from_str_radix(&color_str, 16)
                            .map_err(|_| "Invalid basic color code".to_string())?;
                        current_color = TextComponentColor::Basic(color_value);
                    }
                    // Set color
                    Some('x') => {
                        let mut color_str = String::new();
                        for _ in 0..8 {
                            match chars.next() {
                                Some(c) if c.is_ascii_hexdigit() => color_str.push(c),
                                _ => return Err("Invalid color code".to_string()),
                            }
                        }
                        if color_str.len() != 8 {
                            return Err(format!("Invalid color code: {}", color_str));
                        }
                        let r = u8::from_str_radix(&color_str[0..2], 16)
                            .map_err(|_| "Invalid color code for red channel".to_string())?;
                        let g = u8::from_str_radix(&color_str[2..4], 16)
                            .map_err(|_| "Invalid color code for green channel".to_string())?;
                        let b = u8::from_str_radix(&color_str[4..6], 16)
                            .map_err(|_| "Invalid color code for blue channel".to_string())?;
                        let a = u8::from_str_radix(&color_str[6..8], 16)
                            .map_err(|_| "Invalid color code for alpha channel".to_string())?;
                        current_color = TextComponentColor::Hex(Vec4::new(
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                            a as f32 / 255.0,
                        ));
                    }
                    // Reset color
                    Some('r') => {
                        current_color = TextComponentColor::None;
                    }
                    // Just a normal '%' character
                    Some('%') => current_text.push('%'),
                    None => return Err("Unexpected end of string after '%'".to_string()),
                    _ => return Err("Invalid format code after '%'".to_string()),
                }
            } else {
                current_text.push(c);
            }
        }

        if !current_text.is_empty() {
            parts.push(TextComponentPart {
                text: current_text,
                color: current_color,
            });
        }

        Ok(Self { parts })
    }
}

#[derive(Debug, Default)]
pub struct UniqueQueue<T> {
    queue: std::collections::VecDeque<T>,
    set: std::collections::HashSet<T>,
}

impl<T: std::hash::Hash + Eq + Copy> UniqueQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: std::collections::VecDeque::new(),
            set: std::collections::HashSet::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        if self.set.insert(item) {
            self.queue.push_back(item);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if let Some(item) = self.queue.pop_front() {
            self.set.remove(&item);
            Some(item)
        } else {
            None
        }
    }

    pub fn remove(&mut self, item: &T) {
        if self.set.remove(item)
            && let Some(pos) = self.queue.iter().position(|x| x == item)
        {
            self.queue.remove(pos);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn drain(&mut self, max: usize) -> Vec<T> {
        let mut items = Vec::new();
        for _ in 0..max {
            if let Some(item) = self.pop() {
                items.push(item);
            } else {
                break;
            }
        }
        items
    }
}

pub(crate) fn aabb_overlap(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
    !(a_max.x <= b_min.x
        || a_min.x >= b_max.x
        || a_max.y <= b_min.y
        || a_min.y >= b_max.y
        || a_max.z <= b_min.z
        || a_min.z >= b_max.z)
}

pub(crate) fn ray_intersect_aabb(
    ray_origin: Vec3,
    ray_dir: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<IVec3> {
    let inv_dir = 1.0 / ray_dir;
    let t1 = (aabb_min - ray_origin) * inv_dir;
    let t2 = (aabb_max - ray_origin) * inv_dir;

    let tmin = t1.min(t2);
    let tmax = t1.max(t2);

    let t_enter = tmin.x.max(tmin.y).max(tmin.z);
    let t_exit = tmax.x.min(tmax.y).min(tmax.z);

    if t_enter < t_exit && t_exit > 0.0 {
        // Determine which face was hit based on which component of t_enter is largest
        if t_enter == tmin.x {
            Some(Vec3::new(-inv_dir.x.signum(), 0.0, 0.0).as_ivec3())
        } else if t_enter == tmin.y {
            Some(Vec3::new(0.0, -inv_dir.y.signum(), 0.0).as_ivec3())
        } else {
            Some(Vec3::new(0.0, 0.0, -inv_dir.z.signum()).as_ivec3())
        }
    } else {
        None
    }
}

fn parse_coord(coord: &str, player_coord: f32) -> Result<f32, String> {
    // supports
    // "100"
    // "100.5"
    // "~"
    // "~1"
    // "~1.5"
    if let Some(stripped) = coord.strip_prefix('~') {
        let offset = if stripped.is_empty() {
            0.0
        } else {
            stripped.parse::<f32>().map_err(|_| format!("Invalid coordinate: {}", coord))?
        };
        Ok(player_coord + offset)
    } else {
        coord.parse::<f32>().map_err(|_| format!("Invalid coordinate: {}", coord))
    }
}

pub(crate) fn parse_coords(x: &str, y: &str, z: &str, player_pos: Vec3) -> Result<Vec3, String> {
    Ok(Vec3::new(
        parse_coord(x, player_pos.x)?,
        parse_coord(y, player_pos.y)?,
        parse_coord(z, player_pos.z)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_component_parsing() {
        let input = "Hello %xFF0000FFworld%x00FF00FF!%r Goodbye. %%";
        let component = input.parse::<TextComponent>().unwrap();
        assert_eq!(component.parts.len(), 5);
        assert_eq!(component.parts[0].text, "Hello ");
        assert_eq!(component.parts[0].color, TextComponentColor::None);
        assert_eq!(component.parts[1].text, "world");
        assert_eq!(
            component.parts[1].color,
            TextComponentColor::Hex(Vec4::new(1.0, 0.0, 0.0, 1.0))
        );
        assert_eq!(component.parts[2].text, "!");
        assert_eq!(
            component.parts[2].color,
            TextComponentColor::Hex(Vec4::new(0.0, 1.0, 0.0, 1.0))
        );
        assert_eq!(component.parts[3].text, " Goodbye. ");
        assert_eq!(component.parts[3].color, TextComponentColor::None);
        assert_eq!(component.parts[4].text, "%");
        assert_eq!(component.parts[4].color, TextComponentColor::None);
    }
}
