use std::{io::Cursor, path::PathBuf, sync::Arc};

use fxhash::FxHashMap;
use glam::Vec3;
use mp3d_core::entity::registration::EntityDefId;
use obj::{Obj, TexturedVertex, load_obj};

use crate::{
    abs::{Mesh, Texture},
    render::entities::EntityVertex,
    resource::ResourceManager,
};

impl From<TexturedVertex> for EntityVertex {
    fn from(v: TexturedVertex) -> Self {
        Self {
            position: Vec3::from_array(v.position),
            uv: Vec3::from_array(v.texture).truncate(),
            normal: Vec3::from_array(v.normal),
        }
    }
}

pub struct EntityModel {
    pub mesh: Mesh,
    pub tex: Option<Texture>,
}

pub struct EntityModelLoader {
    map: FxHashMap<Option<EntityDefId>, EntityModel>,
    paths: FxHashMap<(PathBuf, PathBuf), Option<EntityDefId>>,
    finished: bool,
}

impl EntityModelLoader {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            paths: FxHashMap::default(),
            finished: false,
        }
    }

    pub fn insert(&mut self, mesh: PathBuf, tex: PathBuf, kind: Option<EntityDefId>) {
        self.paths.insert((mesh, tex), kind);
    }

    pub fn load_all(
        &mut self,
        resource_manager: &ResourceManager,
        gl: &Arc<glow::Context>,
    ) -> Result<bool, String> {
        if self.finished {
            return Ok(false);
        }

        for ((mesh_p, tex_p), k) in std::mem::take(&mut self.paths) {
            let Some(content) = resource_manager.read(&mesh_p) else {
                return Err(format!("File '{}' doesn't exist", mesh_p.display()));
            };
            let obj: Obj<TexturedVertex, u32> =
                load_obj(Cursor::new(content)).map_err(|e| e.to_string())?;
            let vertices: Vec<_> = obj.vertices.into_iter().map(EntityVertex::from).collect();
            let mesh = Mesh::new(gl, &vertices, &obj.indices, glow::TRIANGLES);
            let tex = resource_manager
                .read(&tex_p)
                .map(|content| {
                    let img = image::load_from_memory(&content).map_err(|e| e.to_string())?;
                    Ok::<Texture, String>(Texture::new(gl, &img))
                })
                .transpose()?;
            self.map.insert(k, EntityModel { mesh, tex });
        }

        Ok(true)
    }

    pub fn get(&self, kind: Option<EntityDefId>) -> Option<&EntityModel> {
        self.map.get(&kind)
    }
}
