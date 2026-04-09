//! Model handling.
//!
//! Reads model files and textures and prepares them for rendering.

pub mod block;
pub mod pack;

pub trait AssetSource: Send + Sync {
    fn read(&self, path: &std::path::Path) -> Option<Vec<u8>>;
}

pub struct EmbeddedAssetSource;

impl AssetSource for EmbeddedAssetSource {
    fn read(&self, path: &std::path::Path) -> Option<Vec<u8>> {
        crate::ASSETS.get_file(path).map(|f| f.contents().to_vec())
    }
}

pub struct FolderAssetSource {
    pub root: std::path::PathBuf,
}

impl AssetSource for FolderAssetSource {
    fn read(&self, path: &std::path::Path) -> Option<Vec<u8>> {
        let full_path = self.root.join(path);
        if let Ok(contents) = std::fs::read(full_path) {
            Some(contents)
        } else {
            log::error!("Failed to read asset file: {}", path.display());
            None
        }
    }
}

pub struct ResourceManager {
    // last has lower priority
    sources: Vec<Box<dyn AssetSource>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            sources: vec![Box::new(EmbeddedAssetSource)],
        }
    }

    pub fn add_source(&mut self, source: Box<dyn AssetSource>) {
        self.sources.push(source);
    }

    pub fn read(&self, path: &std::path::Path) -> Option<Vec<u8>> {
        if self.sources.is_empty() {
            panic!("ResourceManager has no sources");
        }

        for source in self.sources.iter().rev() {
            if let Some(contents) = source.read(path) {
                return Some(contents);
            }
        }
        None
    }
}
