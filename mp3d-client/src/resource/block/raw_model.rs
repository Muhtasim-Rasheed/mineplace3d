use std::collections::HashMap;

use super::TextureRef;

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockModel {
    pub parent: Option<String>,
    pub transform: Option<RawBlockModelTransform>,
    pub elements: Option<Vec<RawBlockElement>>,
    pub textures: Option<HashMap<String, String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockModelTransform {
    /// Rotation is in multiples of 90 degrees
    pub rotation: [u8; 3],
    pub translation: [f32; 3],
    pub scale: [f32; 3],
}

impl std::ops::Mul for RawBlockModelTransform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            rotation: [
                (self.rotation[0] + rhs.rotation[0]) % 4,
                (self.rotation[1] + rhs.rotation[1]) % 4,
                (self.rotation[2] + rhs.rotation[2]) % 4,
            ],
            translation: [
                self.translation[0] + rhs.translation[0],
                self.translation[1] + rhs.translation[1],
                self.translation[2] + rhs.translation[2],
            ],
            scale: [
                self.scale[0] * rhs.scale[0],
                self.scale[1] * rhs.scale[1],
                self.scale[2] * rhs.scale[2],
            ],
        }
    }
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
    pub occludes: Option<bool>,
    pub cullable: Option<bool>,
}
