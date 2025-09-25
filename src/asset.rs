use std::collections::HashMap;

use glam::{Vec2, Vec3};

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
    uvs: Vec<[[[f32; 2]; 2]; 6]>,
}

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub cubes: Vec<[Vec3; 2]>,
    pub uvs: Vec<[[Vec2; 2]; 6]>,
}

#[derive(Debug, Clone)]
pub struct ModelDefs {
    map: HashMap<String, ModelDef>,
}

impl ModelDefs {
    pub fn new(s: &str, atlas_size: Vec2) -> Result<Self, String> {
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
                    let mut transformed = [[Vec2::ZERO; 2]; 6];
                    for (i, uv) in uv_set.iter().enumerate() {
                        transformed[i] = [
                            Vec2::from(uv[0]) / atlas_size,
                            Vec2::from(uv[1]) / atlas_size,
                        ];
                    }
                    transformed
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
