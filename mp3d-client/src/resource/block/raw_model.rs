use std::collections::HashMap;

use super::TextureRef;

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockModel {
    pub parent: Option<String>,
    pub elements: Option<Vec<RawBlockElement>>,
    pub textures: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub n: RawBlockFace,
    pub s: RawBlockFace,
    pub e: RawBlockFace,
    pub w: RawBlockFace,
    pub u: RawBlockFace,
    pub d: RawBlockFace,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockFace {
    pub uv: [f32; 4],
    pub texture: TextureRef,
}
