const BASE_DATAPACK: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/datapack/base");

pub trait DataSource: Send + Sync {
    fn read(&self, path: &std::path::Path) -> Option<Vec<u8>>;
}

struct EmbeddedDataSource;

impl DataSource for EmbeddedDataSource {
    fn read(&self, path: &std::path::Path) -> Option<Vec<u8>> {
        BASE_DATAPACK.get_file(path).map(|f| f.contents().to_vec())
    }
}

pub struct DataSources {
    // last has lower priority
    sources: Vec<Box<dyn DataSource>>,
}

impl Default for DataSources {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSources {
    pub fn new() -> Self {
        Self {
            sources: vec![Box::new(EmbeddedDataSource)],
        }
    }

    pub fn add_source(&mut self, source: Box<dyn DataSource>) {
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

        log::error!("Asset not found: {}", path.display());

        None
    }

    pub fn read_utf8(&self, path: &std::path::Path) -> Option<String> {
        self.read(path)
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }
}
