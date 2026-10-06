use std::path::PathBuf;

use fxhash::FxHashMap;
use glam::Vec3;
use mp3d_core::block::BlockState;

use crate::resource::{
    ResourceManager,
    block::{BlockModel, BlockModelTransform, TextureAtlas},
};

#[derive(Hash, PartialEq, Eq)]
struct TransformKey {
    rotation: [u32; 3],
    translation: [u32; 3],
    scale: [u32; 3],
}

impl From<BlockModelTransform> for TransformKey {
    fn from(transform: BlockModelTransform) -> Self {
        Self {
            rotation: transform.rotation.to_array().map(|v| v.to_bits()),
            translation: transform.translation.to_array().map(|v| v.to_bits()),
            scale: transform.scale.to_array().map(|v| v.to_bits()),
        }
    }
}

impl From<TransformKey> for BlockModelTransform {
    fn from(transform: TransformKey) -> Self {
        Self {
            rotation: Vec3::from_array(transform.rotation.map(|v| f32::from_bits(v))),
            translation: Vec3::from_array(transform.translation.map(|v| f32::from_bits(v))),
            scale: Vec3::from_array(transform.scale.map(|v| f32::from_bits(v))),
        }
    }
}

pub struct BlockModelLoader {
    pub models: Vec<BlockModel>,
    pub map: FxHashMap<BlockState, usize>,
    paths: FxHashMap<(PathBuf, Option<TransformKey>), Vec<BlockState>>,
    finished: bool,
}

impl BlockModelLoader {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            map: FxHashMap::default(),
            paths: FxHashMap::default(),
            finished: false,
        }
    }

    pub fn insert(
        &mut self,
        state: BlockState,
        path: PathBuf,
        transform: Option<BlockModelTransform>,
    ) -> bool {
        let _ = !self.finished || return false; // if finished, return false
        self.paths
            .entry((path, transform.map(TransformKey::from)))
            .or_default()
            .push(state);
        true
    }

    pub fn load_all(
        &mut self,
        resource_manager: &ResourceManager,
        atlas: &mut TextureAtlas,
    ) -> Result<bool, String> {
        let _ = !self.finished || return Ok(false); // if finished, return Ok(false)
        for ((path, transform_key), states) in std::mem::take(&mut self.paths) {
            let model = BlockModel::from_block(
                path,
                transform_key.map(|v| v.into()),
                resource_manager,
                atlas,
            )?;
            let index = self.models.len();
            self.models.push(model);
            for state in states {
                self.map.insert(state, index);
            }
        }
        self.finished = true;
        Ok(true)
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn get(&self, state: BlockState) -> Option<&BlockModel> {
        let index = *self.map.get(&state)?;
        self.models.get(index)
    }

    pub fn finished(&self) -> bool {
        self.finished
    }
}
