use std::{collections::HashMap, path::PathBuf};

use glam::{UVec2, Vec2};

use crate::abs::Texture;

/// Constant size for textures, in pixels.
pub const TEXTURE_SIZE: u32 = 16;

/// A texture reference, either being a slot or the file path to the texture.
#[derive(Clone, Debug, PartialEq)]
pub enum TextureRef {
    Slot(String),
    // The `String` is the path to the texture file, relative to the `blocks/textures` directory
    // and without the `.png` extension.
    File(PathBuf, String),
}

impl std::str::FromStr for TextureRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(slot) = s.strip_prefix('$') {
            Ok(TextureRef::Slot(slot.to_string()))
        } else {
            let p = PathBuf::from("blocks/textures").join(format!("{}.png", s));
            Ok(TextureRef::File(p, s.to_string()))
        }
    }
}

impl<'de> serde::Deserialize<'de> for TextureRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl TextureRef {
    pub fn resolve(&self, slots: &HashMap<String, TextureRef>) -> Option<(PathBuf, String)> {
        let mut visited = std::collections::HashSet::new();
        let mut current = self;
        loop {
            match current {
                TextureRef::Slot(slot) => {
                    if !visited.insert(slot) {
                        return None;
                    }
                    current = slots.get(&format!("${}", slot))?;
                }
                TextureRef::File(path, name) => return Some((path.clone(), name.clone())),
            }
        }
    }
}

/// A texture atlas, both creating a texture and providing UV coordinates for it.
pub struct TextureAtlas {
    pub image: image::RgbaImage,
    width: u32,
    height: u32,
    uv_coords: HashMap<String, [UVec2; 2]>,
    cursor: UVec2,
    gpu_tex: std::sync::OnceLock<Texture>,
}

impl TextureAtlas {
    pub fn new(textures: u32, per_row: u32) -> Self {
        TextureAtlas {
            image: image::RgbaImage::new(
                per_row * TEXTURE_SIZE,
                textures.div_ceil(per_row) * TEXTURE_SIZE,
            ),
            width: per_row * TEXTURE_SIZE,
            height: textures.div_ceil(per_row) * TEXTURE_SIZE,
            uv_coords: HashMap::new(),
            cursor: UVec2::ZERO,
            gpu_tex: std::sync::OnceLock::new(),
        }
    }

    /// Adds a texture to the atlas, returning its UV coordinates in the atlas. Returns `None` if
    /// the atlas is full or has already been uploaded to the GPU. If the texture is already added,
    /// returns the existing UV coordinates.
    pub fn add_texture(&mut self, name: String, texture: image::RgbaImage) -> Option<[UVec2; 2]> {
        if self.cursor.y >= self.height || self.is_finished() {
            return None;
        }

        if let Some(uv) = self.uv_coords.get(&name) {
            return Some(*uv);
        }

        let x = self.cursor.x;
        let y = self.cursor.y;

        if texture.width() != texture.height() {
            log::warn!(
                "Texture '{}' is not square ({}x{})",
                name,
                texture.width(),
                texture.height()
            );
        }
        if texture.width() > TEXTURE_SIZE || texture.height() > TEXTURE_SIZE {
            log::warn!(
                "Texture '{}' is larger than the atlas tile size ({}x{})",
                name,
                texture.width(),
                texture.height()
            );
        }

        image::imageops::replace(
            &mut self.image,
            &image::imageops::resize(
                &texture,
                TEXTURE_SIZE,
                TEXTURE_SIZE,
                image::imageops::FilterType::Nearest,
            ),
            x as i64,
            y as i64,
        );

        self.cursor.x += TEXTURE_SIZE;
        if self.cursor.x >= self.width {
            self.cursor.x = 0;
            self.cursor.y += TEXTURE_SIZE;
        }

        let x = self.cursor.x;
        let y = self.cursor.y;

        let uv_bottom_left = UVec2::new(x, y + TEXTURE_SIZE);
        let uv_top_right = UVec2::new(x + TEXTURE_SIZE, y);
        self.uv_coords.insert(name, [uv_bottom_left, uv_top_right]);

        Some([uv_bottom_left, uv_top_right])
    }

    /// Gets the UV coordinates for a texture in the atlas, if it exists.
    pub fn get_uv(&self, name: &str, model_uv: [Vec2; 2]) -> Option<[Vec2; 2]> {
        let uv = self.uv_coords.get(name)?;
        let w = self.width as f32;
        let h = self.height as f32;
        let atlas_min = uv[0].as_vec2() / Vec2::new(w, h);

        let tile_size = Vec2::new(TEXTURE_SIZE as f32 / w, TEXTURE_SIZE as f32 / h);

        Some([
            atlas_min + model_uv[0] * tile_size,
            atlas_min - model_uv[1] * tile_size,
        ])
    }

    /// Uploads the atlas to the GPU, returning a reference to the GPU texture. If the atlas has
    /// already been uploaded, returns the existing GPU texture reference.
    pub fn upload(&self, gl: &std::sync::Arc<glow::Context>) -> &Texture {
        self.gpu_tex
            .get_or_init(|| Texture::new(gl, &image::DynamicImage::ImageRgba8(self.image.clone())))
    }

    /// Frees the CPU memory used by the atlas image. This should be called after uploading the
    /// atlas to the GPU, as the image data is no longer needed on the CPU side.
    pub fn free_cpu_memory(&mut self) {
        if self.is_finished() {
            self.image = image::RgbaImage::new(0, 0);
        }
    }

    /// Checks if the atlas is finished, meaning that no more textures can be added and it has
    /// already been uploaded to the GPU.
    pub fn is_finished(&self) -> bool {
        self.gpu_tex.get().is_some()
    }

    /// Returns the number of textures currently in the atlas.
    pub fn texture_count(&self) -> usize {
        self.uv_coords.len()
    }
}
