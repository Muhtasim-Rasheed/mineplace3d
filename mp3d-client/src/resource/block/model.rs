use std::{collections::HashMap, path::PathBuf};

use glam::{Mat4, Vec2, Vec3, Vec4};
use mp3d_core::block::Block;

use crate::{render::meshing::FACE_VERTS, resource::ResourceManager};

use super::{
    TextureAtlas, TextureRef,
    raw_model::{RawBlockElement, RawBlockFace, RawBlockModel},
};

/// A block model, containing all the information needed to render a block.
pub struct BlockModel {
    pub elements: Vec<BlockElement>,
}

impl BlockModel {
    /// Creates a [`BlockModel`] from a reference to a [`Block`], using its identifier to find the
    /// corresponding model file and then parsing it. This is a convenience method that combines
    /// loading the raw model from the file and then resolving it to a `BlockModel`.
    pub fn from_block(
        block: &Block,
        extra_ident: &'static str,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Self, String> {
        let path =
            PathBuf::from("blocks/models").join(format!("{}{}.json", block.ident, extra_ident));
        let raw_content = resource_manager.read_utf8(&path).ok_or_else(|| {
            format!(
                "Model file not found for block '{}': {:?}",
                block.ident, path
            )
        })?;
        let raw_model: RawBlockModel = serde_json::from_str(&raw_content).map_err(|e| {
            format!(
                "Failed to parse model JSON for block '{}': {}",
                block.ident, e
            )
        })?;
        Self::from_raw(raw_model, resource_manager, atlas)
    }

    /// Creates a [`BlockModel`] from a [`RawBlockModel`], resolving texture references and parent
    /// models.
    pub fn from_raw(
        raw: RawBlockModel,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Self, String> {
        if atlas.is_finished() {
            return Err("Cannot create block model after texture atlas is finished".to_string());
        }

        let mut textures = HashMap::new();
        if let Some(raw_textures) = raw.textures {
            for (k, v) in raw_textures {
                let texture_ref = v
                    .parse::<TextureRef>()
                    .map_err(|e| format!("Failed to parse texture reference for '{}': {}", k, e))?;
                textures.insert(k, texture_ref);
            }
        }

        let mut elements = Vec::new();

        if let Some(raw_elements) = raw.elements {
            for (i, raw_element) in raw_elements.into_iter().enumerate() {
                let element =
                    BlockElement::from_raw(raw_element, &textures, resource_manager, atlas)
                        .map_err(|e| format!("Failed to resolve element {}: {}", i, e))?;
                elements.push(element);
            }
        } else if let Some(parent) = raw.parent {
            let path = PathBuf::from("blocks/models").join(format!("{}.json", parent));
            elements = Self::resolve_elements(
                &path,
                &mut textures,
                &mut std::collections::HashSet::new(),
                resource_manager,
                atlas,
            )
            .map_err(|e| format!("Failed to resolve parent model '{}': {}", parent, e))?;
        }

        Ok(BlockModel { elements })
    }

    /// Recursively resolves elements from parent models, while also resolving texture references
    /// and detecting circular inheritance.
    fn resolve_elements(
        parent: &PathBuf,
        textures: &mut HashMap<String, TextureRef>,
        visited: &mut std::collections::HashSet<PathBuf>,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Vec<BlockElement>, String> {
        if visited.contains(parent) {
            return Err(format!(
                "Circular model inheritance detected at {:?}",
                parent
            ));
        }

        visited.insert(parent.clone());

        let parent_content = resource_manager
            .read_utf8(parent)
            .ok_or_else(|| format!("Parent model file not found: {:?}", parent))?;
        let parent_raw: RawBlockModel = serde_json::from_str(&parent_content)
            .map_err(|e| format!("Failed to parse parent model JSON: {}", e))?;

        if let Some(parent_textures) = parent_raw.textures {
            for (k, v) in parent_textures {
                let texture_ref = v
                    .parse::<TextureRef>()
                    .map_err(|e| format!("Failed to parse texture reference for '{}': {}", k, e))?;
                textures.insert(k, texture_ref);
            }
        }

        if let Some(elements) = parent_raw.elements {
            let mut resolved_elements = Vec::new();
            for (i, raw_element) in elements.into_iter().enumerate() {
                let element =
                    BlockElement::from_raw(raw_element, textures, resource_manager, atlas)
                        .map_err(|e| format!("Failed to resolve element {}: {}", i, e))?;
                resolved_elements.push(element);
            }
            Ok(resolved_elements)
        } else if let Some(grand_parent) = parent_raw.parent {
            let grand_parent_path =
                PathBuf::from("blocks/models").join(format!("{}.json", grand_parent));
            Self::resolve_elements(
                &grand_parent_path,
                textures,
                visited,
                resource_manager,
                atlas,
            )
        } else {
            Err(format!("Model {:?} has no elements and no parent", parent))
        }
    }

    /// Returns a set of draw commands for the UI renderer to render this block model, given the
    /// texture atlas. Each draw command contains the vertex data for a single face of a block
    /// element, along with the texture coordinates and the texture to use from the atlas.
    pub fn draw_commands(
        &self,
        gl: &std::sync::Arc<glow::Context>,
        atlas: &TextureAtlas,
        position: Vec2,
        size: Vec2,
        rotation: Mat4,
    ) -> Vec<crate::render::ui::uirenderer::DrawCommand> {
        let mut commands = Vec::new();
        for element in &self.elements {
            for (i, face) in element.faces.iter().enumerate() {
                let [uv_min, uv_max] = atlas.get_uv(&face.texture_name, face.uv).unwrap();

                let uvs = [
                    Vec2::new(uv_min.x, uv_max.y),
                    Vec2::new(uv_max.x, uv_max.y),
                    Vec2::new(uv_max.x, uv_min.y),
                    Vec2::new(uv_min.x, uv_min.y),
                ];

                let mut vertices = Vec::new();

                for (vert, uv) in FACE_VERTS[i ^ 1].iter().zip(uvs.iter()) {
                    let from = element.from + 0.5;
                    let to = element.to + 0.5;
                    let rotated = rotation.transform_point3(*vert - (to - from) + from);
                    vertices.push(crate::render::ui::UIVertex {
                        position: (position + rotated.truncate() * size).extend(rotated.z + 2.0),
                        uv: *uv,
                    });
                }

                commands.push(crate::render::ui::uirenderer::DrawCommand::Mesh {
                    vertices,
                    indices: vec![0, 1, 2, 0, 2, 3],
                    mode: crate::render::ui::uirenderer::UIRenderMode::Texture(
                        atlas.upload(gl).handle(),
                        Vec4::ONE,
                    ),
                });
            }
        }
        commands
    }

    /// Returns if this block model contains only one cube element from (0, 0, 0) to (16, 16, 16).
    #[inline]
    pub fn is_full_cube(&self) -> bool {
        self.elements.len() == 1
            && self.elements[0].from == Vec3::ZERO
            && self.elements[0].to == Vec3::splat(1.0)
    }
}

/// A single cuboid element of a block model, defined by two opposite corners and the faces that
/// make up the cuboid. Each face has its own texture and UV coordinates.
pub struct BlockElement {
    pub from: Vec3,
    pub to: Vec3,
    pub faces: [BlockFace; 6],
}

impl BlockElement {
    /// Creates a [`BlockElement`] from a [`RawBlockElement`], resolving texture references and adding
    /// textures to the atlas.
    pub fn from_raw(
        raw: RawBlockElement,
        textures: &HashMap<String, TextureRef>,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Self, String> {
        Ok(BlockElement {
            from: Vec3::from(raw.from) / 16.0,
            to: Vec3::from(raw.to) / 16.0,
            faces: [raw.n, raw.s, raw.e, raw.w, raw.u, raw.d]
                .into_iter()
                .map(|raw_face| BlockFace::from_raw(raw_face, textures, resource_manager, atlas))
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .map_err(|_| "Expected exactly 6 faces".to_string())?,
        })
    }
}

/// A single face of a block element, containing UV coordinates and a texture reference. The
/// texture reference is resolved to a file path and added to the texture atlas when creating a
/// `BlockFace` from a `RawBlockFace`.
pub struct BlockFace {
    pub uv: [Vec2; 2],
    pub texture_name: String,
    pub occludes: bool,
    pub cullable: bool,
}

impl BlockFace {
    /// Creates a [`BlockFace`] from a [`RawBlockFace`], resolving the texture reference and adding the
    /// texture to the atlas. Returns an error if the texture cannot be resolved or if the atlas is
    /// full and cannot accept more textures.
    pub fn from_raw(
        raw: RawBlockFace,
        textures: &HashMap<String, TextureRef>,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Self, String> {
        let texture_path = raw
            .texture
            .resolve(textures)
            .ok_or("Failed to resolve texture")?;
        let image_data = resource_manager
            .read(&texture_path.0)
            .ok_or_else(|| format!("Texture file not found: {:?}", texture_path.0))?;
        let image = image::load_from_memory(&image_data)
            .map_err(|e| format!("Failed to load texture image: {}", e))?
            .to_rgba8();
        atlas
            .add_texture(texture_path.1.clone(), image)
            .ok_or("Atlas is full, cannot add more textures")?;
        Ok(BlockFace {
            uv: [
                Vec2::new(raw.uv[0], raw.uv[1]) / super::TEXTURE_SIZE as f32,
                Vec2::new(raw.uv[2], raw.uv[3]) / super::TEXTURE_SIZE as f32,
            ],
            texture_name: texture_path.1,
            occludes: raw.occludes.unwrap_or(true),
            cullable: raw.cullable.unwrap_or(true),
        })
    }
}
