use std::sync::OnceLock;

use fxhash::FxHashMap;

mod sealed {
    pub struct RegistryToken(());
    impl RegistryToken {
        pub(crate) fn new() -> Self {
            RegistryToken(())
        }
    }
}
pub use sealed::RegistryToken;

pub trait DefId: Copy + Eq + std::hash::Hash {
    fn new(v: usize, _token: RegistryToken) -> Self;
    fn get(&self) -> usize;
}

#[derive(Debug)]
pub struct LazyId<Id: DefId>(OnceLock<Id>);

impl<Id: DefId> LazyId<Id> {
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }

    pub fn set(&self, id: Id) -> Result<(), Id> {
        self.0.set(id)
    }
}

impl<Id: DefId> std::ops::Deref for LazyId<Id> {
    type Target = Id;
    fn deref(&self) -> &Self::Target {
        self.0.get().expect(&format!(
            "ID of type {} was not initialized",
            std::any::type_name::<Id>()
        ))
    }
}

pub trait Def {
    type Id: DefId;

    fn ident(&self) -> &'static str;
}

pub struct DuplicateIdent {
    pub ident: &'static str,
}

pub struct Registry<Entry: Def> {
    entries: Vec<Entry>,
    str_to_id: FxHashMap<&'static str, Entry::Id>,
}

impl<Entry: Def> Registry<Entry> {
    pub fn new() -> Self {
        Registry {
            entries: Vec::new(),
            str_to_id: FxHashMap::default(),
        }
    }

    pub fn register(&mut self, def: Entry) -> Result<Entry::Id, DuplicateIdent> {
        let str_id = def.ident();
        if self.str_to_id.contains_key(str_id) {
            return Err(DuplicateIdent { ident: str_id });
        }
        let id = Entry::Id::new(self.entries.len(), RegistryToken::new());
        self.entries.push(def);
        self.str_to_id.insert(str_id, id);
        Ok(id)
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }

    #[inline]
    pub fn iter_enumerate(&self) -> impl Iterator<Item = (Entry::Id, &Entry)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, entry)| (Entry::Id::new(i, RegistryToken::new()), entry))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    pub fn get_id(&self, str_id: &str) -> Option<Entry::Id> {
        self.str_to_id.get(str_id).copied()
    }

    #[inline]
    pub fn get(&self, id: Entry::Id) -> Option<&Entry> {
        self.entries.get(id.get())
    }
}
