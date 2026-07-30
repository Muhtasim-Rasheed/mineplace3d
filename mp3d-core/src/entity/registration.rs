use crate::{
    entity::EntityDetails,
    registry::{Def, DefId, LazyId, Registry, RegistryToken},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityDefId(usize);

impl DefId for EntityDefId {
    fn new(v: usize, _token: RegistryToken) -> Self {
        Self(v)
    }
    fn get(&self) -> usize {
        self.0
    }
}

pub struct EntityDef {
    pub ident: &'static str,
    pub template: fn() -> EntityDetails,
}

impl Def for EntityDef {
    type Id = EntityDefId;
    fn ident(&self) -> &'static str {
        self.ident
    }
}

pub type EntityRegistry = Registry<EntityDef>;

static ENTITY_REGISTRY: std::sync::OnceLock<EntityRegistry> = std::sync::OnceLock::new();

pub fn entity_registry() -> &'static EntityRegistry {
    ENTITY_REGISTRY
        .get()
        .expect("entity registry not initialized")
}

pub struct EntityRegistration {
    pub build: fn() -> EntityDef,
    pub id_slot: &'static LazyId<EntityDefId>,
}
inventory::collect!(EntityRegistration);

pub fn init_entity_registry() {
    let mut registry = EntityRegistry::new();
    for reg in inventory::iter::<EntityRegistration> {
        let def = (reg.build)();
        let def_ident = def.ident;
        let id = registry
            .register(def)
            .unwrap_or_else(|e| panic!("duplicate entity ident: {}", e.ident));
        reg.id_slot
            .set(id)
            .unwrap_or_else(|_| panic!("entity static for {} set twice", def_ident));
    }
    ENTITY_REGISTRY
        .set(registry)
        .unwrap_or_else(|_| panic!("init_entity_registry called twice"));
}

#[macro_export]
macro_rules! define_entities {
    (
        $(
            $name:ident => {
                ident: $ident:expr,
                template: $template:expr
                $(,)?
            }
        ),* $(,)?
    ) => {
        pub mod entities {
            $(
                pub static $name: $crate::registry::LazyId<$crate::entity::registration::EntityDefId> = $crate::registry::LazyId::new();

                ::inventory::submit! {
                    $crate::entity::registration::EntityRegistration {
                        build: || $crate::entity::registration::EntityDef {
                            ident: $ident,
                            template: $template,
                        },
                        id_slot: &$name,
                    }
                }
            )*
        }
    };
}
