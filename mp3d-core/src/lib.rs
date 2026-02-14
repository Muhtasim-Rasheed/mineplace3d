//! The core of the Mineplace3D engine. This module contains essential
//! functionalities and definitions required for the engine to operate
//! such as world management, entity handling, etc.

use glam::Vec4;

pub mod block;
pub mod entity;
pub mod protocol;
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
                Vec4::new(r as f32 / 3.0, g as f32 / 3.0, b as f32 / 3.0, (a as f32 + 1.0) / 4.0)
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
                        current_color = TextComponentColor::Hex(Vec4::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0));
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
