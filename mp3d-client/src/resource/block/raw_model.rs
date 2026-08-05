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
    pub rotation: Option<[f32; 3]>,
    pub translation: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    /// An omitted direction has no geometry. This lets models describe open cuboids and
    /// planes without supplying dummy, invisible faces.
    pub n: Option<RawBlockFace>,
    pub s: Option<RawBlockFace>,
    pub e: Option<RawBlockFace>,
    pub w: Option<RawBlockFace>,
    pub u: Option<RawBlockFace>,
    pub d: Option<RawBlockFace>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RawBlockFace {
    pub uv: [f32; 4],
    pub texture: TextureRef,
    pub occludes: Option<bool>,
    pub cullable: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::RawBlockElement;

    #[test]
    fn accepts_elements_with_omitted_faces() {
        let element: RawBlockElement =
            serde_json::from_str(r#"{"from":[0.0,0.0,0.0],"to":[16.0,16.0,16.0]}"#).unwrap();

        assert!(element.n.is_none());
        assert!(element.s.is_none());
        assert!(element.e.is_none());
        assert!(element.w.is_none());
        assert!(element.u.is_none());
        assert!(element.d.is_none());
    }
}
