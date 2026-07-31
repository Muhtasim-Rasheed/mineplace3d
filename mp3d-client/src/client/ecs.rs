use fxhash::FxHashMap;
use mp3d_core::entity::{EntityDetails, EntityId, components::Component};

pub struct ClientEcs {
    entities: FxHashMap<EntityId, EntityDetails>,
}

impl ClientEcs {
    pub fn new() -> Self {
        Self {
            entities: FxHashMap::default(),
        }
    }

    pub fn spawn(&mut self, id: EntityId, details: EntityDetails) {
        self.entities.insert(id, details);
    }

    pub fn apply_update(&mut self, id: EntityId, details: &EntityDetails) {
        if let Some(existing) = self.entities.get_mut(&id) {
            existing.merge(details);
        }
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    pub fn get<T: Component>(&self, id: EntityId) -> Option<T> {
        self.entities.get(&id)?.get::<T>()
    }

    pub fn query<Q: ClientQuery>(&self) -> impl Iterator<Item = (EntityId, Q::Item)> + '_ {
        self.entities
            .iter()
            .filter_map(|(id, details)| Some((*id, Q::fetch(details)?)))
    }
}

pub trait ClientQuery {
    type Item;
    fn fetch(details: &EntityDetails) -> Option<Self::Item>;
}

impl<T: Component> ClientQuery for &T {
    type Item = T;
    fn fetch(details: &EntityDetails) -> Option<Self::Item> {
        details.get::<T>()
    }
}

macro_rules! impl_client_query_tuple {
    ($($t:ident),+) => {
        impl<$($t: ClientQuery),+> ClientQuery for ($($t),+) {
            type Item = ($($t::Item),+);
            fn fetch(details: &EntityDetails) -> Option<Self::Item> {
                Some(($($t::fetch(details)?),+))
            }
        }
    };
}
impl_client_query_tuple!(A, B);
impl_client_query_tuple!(A, B, C);
impl_client_query_tuple!(A, B, C, D);
impl_client_query_tuple!(A, B, C, D, E);
