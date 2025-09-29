use std::collections::HashMap;

use glam::{UVec2, Vec3};

use crate::{shader::ShaderProgram, texture::Texture};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyPart {
    Numeric(u32),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub parts: Vec<KeyPart>,
}

impl std::str::FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = Vec::new();
        for part in s.split('.') {
            if let Ok(num) = u32::from_str_radix(part, 16) {
                parts.push(KeyPart::Numeric(num));
            } else {
                parts.push(KeyPart::Text(part.to_string()));
            }
        }
        if !matches!(parts.first(), Some(KeyPart::Text(_))) {
            return Err("Path must start with a text part to define the category".to_string());
        }
        Ok(Key { parts })
    }
}

#[derive(Debug)]
pub struct Translations {
    map: HashMap<Key, String>,
}

impl Translations {
    pub fn new(s: &str) -> Result<Self, String> {
        let raw: HashMap<String, String> = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let map = raw
            .into_iter()
            .map(|(k, v)| {
                let path = k.parse::<Key>()?;
                Ok((path, v))
            })
            .collect::<Result<HashMap<_, _>, String>>()?;
        Ok(Translations { map })
    }

    pub fn get(&self, path: Key) -> Option<&String> {
        self.map.get(&path)
    }
}

#[derive(Debug, serde::Deserialize)]
struct RawModelDef {
    includes: Vec<String>,
    cubes: Vec<[[f32; 3]; 2]>,
    uvs: Vec<[[[u32; 2]; 2]; 6]>,
}

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub cubes: Vec<[Vec3; 2]>,
    pub uvs: Vec<[[UVec2; 2]; 6]>,
}

#[derive(Debug, Clone)]
pub struct ModelDefs {
    map: HashMap<String, ModelDef>,
}

impl ModelDefs {
    pub fn new(s: &str) -> Result<Self, String> {
        let raw: indexmap::IndexMap<String, RawModelDef> =
            serde_json::from_str(s).map_err(|e| e.to_string())?;
        let mut map: HashMap<String, ModelDef> = HashMap::new();
        for (name, raw_def) in raw {
            let mut cubes: Vec<_> = raw_def
                .cubes
                .into_iter()
                .map(|[min, max]| [Vec3::from(min), Vec3::from(max)])
                .collect();
            let mut uvs: Vec<_> = raw_def
                .uvs
                .into_iter()
                .map(|uv_set| {
                    let mut vecs = [[UVec2::ZERO; 2]; 6];
                    for (i, uv) in uv_set.iter().enumerate() {
                        vecs[i] = [UVec2::from(uv[0]), UVec2::from(uv[1])];
                    }
                    vecs
                })
                .collect();
            for include in raw_def.includes {
                if let Some(included) = map.get(&include) {
                    cubes.extend(included.cubes.iter().cloned());
                    uvs.extend(included.uvs.iter().cloned());
                } else {
                    return Err(format!(
                        "Included model '{}' not found for model '{}'",
                        include, name
                    ));
                }
            }
            map.insert(name, ModelDef { cubes, uvs });
        }
        Ok(ModelDefs { map })
    }

    pub fn get(&self, name: &str) -> Option<&ModelDef> {
        self.map.get(name)
    }
}

// #[derive(Default)]
// pub struct ResourceManager {
//     translations: Option<Translations>,
//     model_defs: Option<ModelDefs>,
//     textures: HashMap<String, Texture>,
//     shader_programs: HashMap<String, ShaderProgram>,
// }

// impl ResourceManager {
//     pub fn new() -> Self {
//         ResourceManager::default()
//     }

//     pub fn with_translations(mut self, s: &str) -> Result<Self, String> {
//         self.translations = Some(Translations::new(s)?);
//         Ok(self)
//     }

//     pub fn with_model_defs(mut self, s: &str) -> Result<Self, String> {
//         self.model_defs = Some(ModelDefs::new(s)?);
//         Ok(self)
//     }

//     pub fn add_texture(mut self, name: &str, texture: Texture) -> Self {
//         self.textures.insert(name.to_string(), texture);
//         self
//     }

//     pub fn add_shader_program(mut self, name: &str, program: ShaderProgram) -> Self {
//         self.shader_programs.insert(name.to_string(), program);
//         self
//     }

//     pub fn get_texture(&self, name: &str) -> Option<&Texture> {
//         self.textures.get(name)
//     }

//     pub fn get_shader_program(&self, name: &str) -> Option<&ShaderProgram> {
//         self.shader_programs.get(name)
//     }
// }

pub trait Resource: 'static + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

macro_rules! impl_resource {
    ($t:ty) => {
        impl Resource for $t {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

impl_resource!(Translations);
impl_resource!(ModelDefs);
impl_resource!(Texture);
impl_resource!(ShaderProgram);

pub struct ResourceManager {
    resources: HashMap<String, Box<dyn Resource>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {
            resources: HashMap::new(),
        }
    }

    pub fn add<R: Resource>(mut self, name: &str, resource: R) -> Self {
        self.resources.insert(name.to_string(), Box::new(resource));
        self
    }

    pub fn get<R: Resource>(&self, name: &str) -> Option<&R> {
        self.resources.get(name)?.as_any().downcast_ref::<R>()
    }
}
