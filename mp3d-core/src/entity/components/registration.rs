use std::any::Any;

use crate::{
    entity::components::ErasedStorage,
    registry::{Def, DefId, LazyId, Registry, RegistryToken},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(usize);

impl DefId for ComponentId {
    fn new(v: usize, _token: RegistryToken) -> Self {
        Self(v)
    }
    fn get(&self) -> usize {
        self.0
    }
}

pub struct ComponentDef {
    pub ident: &'static str,
    pub serialize: fn(&dyn Any) -> Vec<u8>,
    pub deserialize: fn(&[u8]) -> Box<dyn Any + Send + Sync>,
    pub make_storage: fn() -> Box<dyn ErasedStorage>,
}

impl Def for ComponentDef {
    type Id = ComponentId;
    fn ident(&self) -> &'static str {
        self.ident
    }
}

pub type ComponentRegistry = Registry<ComponentDef>;

static COMPONENT_REGISTRY: std::sync::OnceLock<ComponentRegistry> = std::sync::OnceLock::new();

pub fn component_registry() -> &'static ComponentRegistry {
    COMPONENT_REGISTRY
        .get()
        .expect("component registry not initialized")
}

pub struct ComponentRegistration {
    pub build: fn() -> ComponentDef,
    pub id_slot: &'static LazyId<ComponentId>,
}
inventory::collect!(ComponentRegistration);

pub fn init_component_registry() {
    let mut registry = ComponentRegistry::new();
    for reg in inventory::iter::<ComponentRegistration> {
        let def = (reg.build)();
        let def_ident = def.ident;
        let id = registry
            .register(def)
            .unwrap_or_else(|e| panic!("duplicate component ident: {}", e.ident));
        reg.id_slot
            .set(id)
            .unwrap_or_else(|_| panic!("component static for {} set twice", def_ident));
    }
    COMPONENT_REGISTRY
        .set(registry)
        .unwrap_or_else(|_| panic!("init_component_registry called twice"));
}

#[macro_export]
macro_rules! impl_component {
    (
        name: $name:ident,
        ident: $ident:literal,
        ty: $ty:ty,
        $to_bytes:item
        $from_bytes:item
    ) => {
        impl $crate::entity::components::Component for $ty {
            fn component_id() -> $crate::entity::components::ComponentId {
                *$name
            }
            $to_bytes
            $from_bytes
        }

        pub static $name: $crate::registry::LazyId<$crate::entity::components::registration::ComponentId> =
            $crate::registry::LazyId::new();

        ::inventory::submit! {
            $crate::entity::components::registration::ComponentRegistration {
                build: || $crate::entity::components::registration::ComponentDef {
                    ident: $ident,
                    serialize: |any: &dyn ::std::any::Any| {
                        $crate::entity::components::Component::to_bytes(
                            any.downcast_ref::<$ty>().expect("component type mismatch")
                        )
                    },
                    deserialize: |bytes| Box::new(<$ty as $crate::entity::components::Component>::from_bytes(bytes)),
                    make_storage: || Box::new($crate::entity::components::ComponentStorage::<$ty>::new()) as Box<dyn ErasedStorage>,
                },
                id_slot: &$name,
            }
        }
    };
}
