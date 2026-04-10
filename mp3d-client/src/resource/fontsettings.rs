#[derive(Debug, Clone, serde::Deserialize)]
pub struct FontSettings {
    pub char_width: u32,
    pub char_height: u32,
    pub first_char: char,
    pub strikethrough_idx: Option<u32>,
}
