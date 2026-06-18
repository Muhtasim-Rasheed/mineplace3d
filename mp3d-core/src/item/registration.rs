use crate::{
    block::BlockId,
    registry::{Def, DefId, LazyId, Registry, RegistryToken},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(usize);

impl DefId for ItemId {
    fn new(v: usize, _token: RegistryToken) -> Self {
        Self(v)
    }

    fn get(&self) -> usize {
        self.0
    }
}

pub struct ItemDef {
    pub ident: &'static str,
    pub assoc_block: Option<&'static LazyId<BlockId>>,
    pub max_stack: u16,
}

impl Def for ItemDef {
    type Id = ItemId;
    fn ident(&self) -> &'static str {
        self.ident
    }
}

pub type ItemRegistry = Registry<ItemDef>;

static ITEM_REGISTRY: std::sync::OnceLock<ItemRegistry> = std::sync::OnceLock::new();

#[inline]
pub fn item_registry() -> &'static ItemRegistry {
    ITEM_REGISTRY
        .get()
        .expect("block registry not initialized - call init_item_registry() first")
}

pub struct ItemRegistration {
    pub build: fn() -> ItemDef,
    pub id_slot: &'static LazyId<ItemId>,
}

inventory::collect!(ItemRegistration);

pub fn init_item_registry() {
    let mut registry = ItemRegistry::new();

    for reg in inventory::iter::<ItemRegistration> {
        let def = (reg.build)();
        let def_ident = def.ident;
        let id = registry
            .register(def)
            .unwrap_or_else(|e| panic!("duplicate item ident: {}", e.ident));
        reg.id_slot
            .set(id)
            .unwrap_or_else(|_| panic!("item static for {} set twice", def_ident));
    }

    ITEM_REGISTRY
        .set(registry)
        .unwrap_or_else(|_| panic!("init_item_registry called twice"));
}

#[macro_export]
macro_rules! define_items {
    (
        $(
            $name:ident => {
                ident: $ident:expr
                $(, block: $assoc_block:expr)?
                $(, max_stack: $max_stack:expr)?
                $(,)?
            }
        ),* $(,)?
    ) => {
        pub mod items {
            use super::*;

            $(
                pub static $name: $crate::registry::LazyId<ItemId> = $crate::registry::LazyId::new();

                ::inventory::submit! {
                    $crate::item::ItemRegistration {
                        build: || ItemDef {
                            ident: $ident,
                            assoc_block: define_items!(@assoc_block $( $assoc_block )?),
                            max_stack: define_items!(@max_stack $( $max_stack )?),
                        },
                        id_slot: &$name,
                    }
                }
            )*
        }
    };

    (@assoc_block $assoc_block:expr) => { Some(&$assoc_block) };
    (@assoc_block) => { None };

    (@max_stack $max_stack:expr) => { $max_stack };
    (@max_stack) => { 64 };
}
