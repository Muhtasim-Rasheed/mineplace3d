use std::{collections::HashMap, path::PathBuf};

use glam::{Affine3A, Mat4, Vec2, Vec3, Vec4};
use mp3d_core::direction::Direction;

use crate::resource::{ResourceManager, block::raw_model::RawBlockModelTransform};

use super::{
    TextureAtlas, TextureRef,
    raw_model::{RawBlockElement, RawBlockFace, RawBlockModel},
};

fn face_corners(from: Vec3, to: Vec3, face: Direction) -> [Vec3; 4] {
    match face {
        Direction::North => [
            glam::vec3(from.x, to.y, from.z),
            glam::vec3(to.x, to.y, from.z),
            glam::vec3(to.x, from.y, from.z),
            glam::vec3(from.x, from.y, from.z),
        ],
        Direction::South => [
            glam::vec3(to.x, to.y, to.z),
            glam::vec3(from.x, to.y, to.z),
            glam::vec3(from.x, from.y, to.z),
            glam::vec3(to.x, from.y, to.z),
        ],
        Direction::East => [
            glam::vec3(to.x, to.y, from.z),
            glam::vec3(to.x, to.y, to.z),
            glam::vec3(to.x, from.y, to.z),
            glam::vec3(to.x, from.y, from.z),
        ],
        Direction::West => [
            glam::vec3(from.x, to.y, to.z),
            glam::vec3(from.x, to.y, from.z),
            glam::vec3(from.x, from.y, from.z),
            glam::vec3(from.x, from.y, to.z),
        ],
        Direction::Up => [
            glam::vec3(to.x, to.y, from.z),
            glam::vec3(from.x, to.y, from.z),
            glam::vec3(from.x, to.y, to.z),
            glam::vec3(to.x, to.y, to.z),
        ],
        Direction::Down => [
            glam::vec3(to.x, from.y, to.z),
            glam::vec3(from.x, from.y, to.z),
            glam::vec3(from.x, from.y, from.z),
            glam::vec3(to.x, from.y, from.z),
        ],
    }
}

/// A block model, containing all the information needed to render a block.
pub struct BlockModel {
    pub elements: Vec<BlockElement>,
    pub particle: Option<String>,
    is_full_cube: bool,
}

impl BlockModel {
    /// Creates a [`BlockModel`] from a reference to a [`Block`], using its identifier to find the
    /// corresponding model file and then parsing it. This is a convenience method that combines
    /// loading the raw model from the file and then resolving it to a `BlockModel`.
    pub fn from_block(
        model_path: PathBuf,
        model_file: &str,
        transform: Option<BlockModelTransform>,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Self, String> {
        let raw_model: RawBlockModel = serde_json::from_str(model_file).map_err(|e| {
            format!(
                "Failed to parse model JSON for block '{}': {}",
                model_path.display(),
                e
            )
        })?;
        Self::from_raw(raw_model, transform, resource_manager, atlas)
    }

    /// Creates a [`BlockModel`] from a [`RawBlockModel`], resolving texture references and parent
    /// models.
    pub fn from_raw(
        raw: RawBlockModel,
        state_transform: Option<BlockModelTransform>,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<Self, String> {
        if atlas.is_finished() {
            return Err("Cannot create block model after texture atlas is finished".to_string());
        }

        let is_full_cube = Self::is_raw_full_cube(
            &raw,
            resource_manager,
            &mut std::collections::HashSet::new(),
        )?;

        let mut textures = HashMap::new();
        if let Some(raw_textures) = raw.textures {
            for (k, v) in raw_textures {
                let texture_ref = v
                    .parse::<TextureRef>()
                    .map_err(|e| format!("Failed to parse texture reference for '{}': {}", k, e))?;
                textures.insert(k, texture_ref);
            }
        }

        let mut transform = raw.transform.map(BlockModelTransform::from);
        if let Some(state_transform) = state_transform {
            transform = Some(if let Some(raw_transform) = transform {
                state_transform * raw_transform
            } else {
                state_transform
            });
        }

        let mut elements = Vec::new();

        if let Some(raw_elements) = raw.elements {
            for (i, raw_element) in raw_elements.into_iter().enumerate() {
                let element = BlockElement::from_raw(
                    raw_element,
                    &textures,
                    resource_manager,
                    atlas,
                    transform,
                )
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
                transform,
            )
            .map_err(|e| format!("Failed to resolve parent model '{}': {}", parent, e))?;
        }

        let particle = textures
            .get("$particle")
            .and_then(|tex_ref| tex_ref.resolve(&textures))
            .map(|(_, name)| name);

        Ok(BlockModel {
            elements,
            particle,
            is_full_cube,
        })
    }

    fn is_raw_full_cube(
        raw: &RawBlockModel,
        resource_manager: &ResourceManager,
        visited: &mut std::collections::HashSet<PathBuf>,
    ) -> Result<bool, String> {
        if let Some(elements) = &raw.elements {
            if raw.transform.is_some() {
                return Ok(false);
            }
            Ok(elements.len() == 1
                && elements[0].from == [0.0, 0.0, 0.0]
                && elements[0].to == [16.0, 16.0, 16.0])
        } else if let Some(parent) = &raw.parent {
            let path = PathBuf::from("blocks/models").join(format!("{}.json", parent));
            if visited.contains(&path) {
                return Err(format!("Circular model inheritance detected at {:?}", path));
            }
            visited.insert(path.clone());
            if let Some(parent_content) = resource_manager.read_utf8(&path) {
                if let Ok(parent_raw) = serde_json::from_str::<RawBlockModel>(&parent_content) {
                    return Self::is_raw_full_cube(&parent_raw, resource_manager, visited);
                }
            }
            Ok(false)
        } else {
            Ok(false)
        }
    }

    /// Recursively resolves elements from parent models, while also resolving texture references
    /// and detecting circular inheritance.
    fn resolve_elements(
        parent: &PathBuf,
        textures: &mut HashMap<String, TextureRef>,
        visited: &mut std::collections::HashSet<PathBuf>,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
        transform: Option<BlockModelTransform>,
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
                let combined_transform = if let Some(parent_transform) = parent_raw.transform {
                    let parent_transform = BlockModelTransform::from(parent_transform);

                    if let Some(current_transform) = transform {
                        Some(parent_transform * current_transform)
                    } else {
                        Some(parent_transform)
                    }
                } else {
                    transform
                };
                let element = BlockElement::from_raw(
                    raw_element,
                    textures,
                    resource_manager,
                    atlas,
                    combined_transform,
                )
                .map_err(|e| format!("Failed to resolve element {}: {}", i, e))?;
                resolved_elements.push(element);
            }
            Ok(resolved_elements)
        } else if let Some(grand_parent) = parent_raw.parent {
            let grand_parent_path =
                PathBuf::from("blocks/models").join(format!("{}.json", grand_parent));
            let combined_transform = if let Some(parent_transform) = parent_raw.transform {
                let parent_transform = BlockModelTransform::from(parent_transform);

                if let Some(current_transform) = transform {
                    Some(parent_transform * current_transform)
                } else {
                    Some(parent_transform)
                }
            } else {
                transform
            };
            Self::resolve_elements(
                &grand_parent_path,
                textures,
                visited,
                resource_manager,
                atlas,
                combined_transform,
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
            for face in element.faces.iter() {
                let [uv_min, uv_max] = atlas.get_uv(&face.texture_name, face.uv).unwrap();

                let uvs = [
                    Vec2::new(uv_max.x, uv_max.y),
                    Vec2::new(uv_min.x, uv_max.y),
                    Vec2::new(uv_min.x, uv_min.y),
                    Vec2::new(uv_max.x, uv_min.y),
                ];

                let mut vertices = Vec::new();

                let corners = face.vertices;
                for (vert, uv) in corners.iter().zip(uvs.iter()) {
                    let rotated = rotation.transform_point3(*vert);
                    let p = Vec2::new(rotated.x, -rotated.y);
                    let normal = rotation.transform_vector3(face.normal);
                    vertices.push(crate::render::ui::UIVertex {
                        position: (position + p * size).extend(rotated.z + 2.0),
                        uv: *uv,
                        // for shading this block we need to specify the normal.
                        normal,
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
        self.is_full_cube
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockModelTransform {
    pub rotation: Vec3,
    pub translation: Vec3,
    pub scale: Vec3,
}

impl From<BlockModelTransform> for glam::Affine3A {
    fn from(transform: BlockModelTransform) -> Self {
        let center = glam::Vec3::splat(0.5);
        let to_origin = glam::Affine3A::from_translation(-center);
        let from_origin = glam::Affine3A::from_translation(center);
        let rotation = glam::Affine3A::from_rotation_x(transform.rotation.x)
            * glam::Affine3A::from_rotation_y(transform.rotation.y)
            * glam::Affine3A::from_rotation_z(transform.rotation.z);
        let scale = glam::Affine3A::from_scale(transform.scale);
        let translation = glam::Affine3A::from_translation(transform.translation);
        from_origin * rotation * scale * to_origin * translation
    }
}

impl From<RawBlockModelTransform> for BlockModelTransform {
    fn from(raw: RawBlockModelTransform) -> Self {
        Self {
            rotation: raw
                .rotation
                .map_or(Vec3::ZERO, |r| Vec3::from_array(r.map(|v| v.to_radians()))),
            translation: raw.translation.map_or(Vec3::ZERO, Vec3::from_array),
            scale: raw.scale.map_or(Vec3::ONE, Vec3::from_array),
        }
    }
}

impl std::ops::Mul for BlockModelTransform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            rotation: (self.rotation + rhs.rotation) % Vec3::splat(std::f32::consts::TAU),
            translation: self.translation + rhs.translation,
            scale: self.scale * rhs.scale,
        }
    }
}

/// A single cuboid element of a block model, defined by two opposite corners and the faces that
/// make up the cuboid. Each face has its own texture and UV coordinates.
pub struct BlockElement {
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
        transform: Option<BlockModelTransform>,
    ) -> Result<Self, String> {
        let from = Vec3::from(raw.from) / 16.0;
        let to = Vec3::from(raw.to) / 16.0;

        let faces: [BlockFace; 6] = [raw.n, raw.s, raw.e, raw.w, raw.u, raw.d]
            .into_iter()
            .enumerate()
            .map(|(i, raw_face)| {
                BlockFace::from_raw(
                    raw_face,
                    textures,
                    resource_manager,
                    atlas,
                    (from, to),
                    Direction::try_from(i as u8).unwrap(),
                    transform,
                )
            })
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| "Expected exactly 6 faces".to_string())?;

        Ok(BlockElement { faces })
    }
}

/// A single face of a block element, containing UV coordinates and a texture reference. The
/// texture reference is resolved to a file path and added to the texture atlas when creating a
/// `BlockFace` from a `RawBlockFace`.
pub struct BlockFace {
    pub vertices: [Vec3; 4],
    pub uv: [Vec2; 2],
    pub normal: Vec3,
    pub texture_name: String,
    pub occludes: bool,
    pub cullable: bool,
    pub occlusion_face: Option<OcclusionFace>,
}

pub struct OcclusionFace {
    pub rect: [Vec2; 2],
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
        aabb: (Vec3, Vec3),
        face: Direction,
        transform: Option<BlockModelTransform>,
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

        let transform = transform.map_or(Affine3A::IDENTITY, Into::into);
        let vertices =
            face_corners(aabb.0, aabb.1, face).map(|corner| transform.transform_point3(corner));
        let normal = transform.transform_vector3(face.into());
        let uv = [
            Vec2::from_slice(&raw.uv[0..2]) / super::TEXTURE_SIZE as f32,
            Vec2::from_slice(&raw.uv[2..4]) / super::TEXTURE_SIZE as f32,
        ];
        let mut occlusion_face = None;
        if Direction::try_from(normal).is_ok() {
            let rect = [Vec2::new(aabb.0.x, aabb.0.y), Vec2::new(aabb.1.x, aabb.1.y)];
            occlusion_face = Some(OcclusionFace { rect });
        }
        Ok(BlockFace {
            vertices,
            uv,
            normal,
            texture_name: texture_path.1,
            occludes: raw.occludes.unwrap_or(true),
            cullable: raw.cullable.unwrap_or(true),
            occlusion_face,
        })
    }
}
