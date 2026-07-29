use std::cell::UnsafeCell;

use glam::Vec3;

use crate::{
    entity::{
        EntityDetails, EntityId,
        components::{
            Component, ComponentId, ComponentStorage, ErasedStorage, HasDefId, Position,
            component_registry,
        },
        registration::{EntityDefId, entity_registry},
    },
    registry::DefId,
    world::World,
};

#[derive(Default)]
pub struct EntityAllocator {
    generations: Vec<u32>,
    free: Vec<u32>,
    alive: Vec<EntityId>,
}

impl EntityAllocator {
    pub fn spawn(&mut self) -> EntityId {
        let index = self.free.pop().unwrap_or_else(|| {
            self.generations.push(0);
            self.generations.len() as u32 - 1
        });
        let id = EntityId {
            index,
            generation: self.generations[index as usize],
        };
        self.alive.push(id);
        id
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        if !self.is_alive(id) {
            return false;
        }
        self.generations[id.index as usize] += 1;
        self.free.push(id.index);
        self.alive
            .swap_remove(self.alive.iter().position(|&e| e == id).unwrap());
        true
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        self.generations.get(id.index as usize) == Some(&id.generation)
    }

    pub fn iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.alive.iter().copied()
    }
}

pub struct ECS {
    pub e_alloc: EntityAllocator,
    storages: Vec<UnsafeCell<Box<dyn ErasedStorage>>>,
}

impl ECS {
    pub fn new() -> Self {
        let registry = component_registry();
        let mut storages = Vec::with_capacity(registry.len());
        for i in 0..registry.len() {
            let id = registry.valid_id(i).expect("registry/index out of sync");
            let def = registry.get(id).unwrap();
            storages.push(UnsafeCell::new((def.make_storage)()));
        }
        Self {
            e_alloc: EntityAllocator::default(),
            storages,
        }
    }

    unsafe fn get_unchecked<T: Component>(&self) -> &ComponentStorage<T> {
        let cell = &self.storages[T::component_id().get()];
        (unsafe { &*cell.get() })
            .as_any()
            .downcast_ref::<ComponentStorage<T>>()
            .expect("registry/ECS out of sync")
    }

    unsafe fn get_unchecked_mut<T: Component>(&self) -> &mut ComponentStorage<T> {
        let cell = &self.storages[T::component_id().get()];
        (unsafe { &mut *cell.get() })
            .as_any_mut()
            .downcast_mut::<ComponentStorage<T>>()
            .expect("registry/ECS out of sync")
    }

    pub fn spawn_entity(&mut self, def_id: EntityDefId, pos: Vec3) -> EntityId {
        let def = entity_registry().get(def_id).unwrap();
        let details = (def.template)();
        let entity = self.spawn_from_details(&details);
        // SAFETY: The mutable borrow is dropped at the end of the statement, so it's fine to create
        // a new mutable borrow.
        unsafe {
            self.get_unchecked_mut::<Position>()
                .insert(entity, Position(pos));
            self.get_unchecked_mut::<HasDefId>()
                .insert(entity, HasDefId(def_id));
        }
        entity
    }

    pub fn spawn_from_details(&mut self, details: &EntityDetails) -> EntityId {
        let entity = self.e_alloc.spawn();
        for (id, data) in &details.components {
            if let Err(e) = self.storages[id.get()].get_mut().apply(entity, data) {
                log::warn!("failed to apply component during spawn: {e}");
            }
        }
        entity
    }

    pub fn despawn(&mut self, entity: EntityId) -> bool {
        if !self.e_alloc.is_alive(entity) {
            return false;
        }
        for storage in &mut self.storages {
            storage.get_mut().remove(entity);
        }
        self.e_alloc.despawn(entity)
    }

    pub fn entity_details(&self, entity: EntityId) -> EntityDetails {
        let registry = component_registry();
        let mut components = Vec::new();
        for i in 0..self.storages.len() {
            if let Some(id) = registry.valid_id(i) {
                // SAFETY: read-only, one storage at a time
                if let Some(bytes) = unsafe { (*self.storages[i].get()).snapshot(entity) } {
                    components.push((id, bytes));
                }
            }
        }
        EntityDetails { components }
    }

    pub fn serialize_entity(&self, entity: EntityId) -> Vec<u8> {
        self.entity_details(entity).to_bytes()
    }

    pub fn query_ro<Q: Query>(&self) -> QueryIter<'_, Q> {
        const {
            assert!(
                Q::READ_ONLY,
                "use query_mut for queries containing &mut components"
            )
        };
        let mut accesses = Vec::new();
        Q::accesses(&mut accesses);
        check_no_conflicts(&accesses);
        QueryIter {
            ecs: self,
            ids: self.e_alloc.iter().collect::<Vec<_>>().into_iter(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn query<Q: Query>(&mut self) -> QueryIter<'_, Q> {
        let mut accesses = Vec::new();
        Q::accesses(&mut accesses);
        check_no_conflicts(&accesses);
        QueryIter {
            ecs: self,
            ids: self.e_alloc.iter().collect::<Vec<_>>().into_iter(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        // SAFETY: `&self` here is a single shared borrow of the whole ECS,
        // not part of a Query's multi-storage access. No other unchecked
        // accessor is live concurrently with this one.
        unsafe { self.get_unchecked::<T>().get(entity) }
    }

    pub fn get_component_copied<T: Component + Copy>(&self, entity: EntityId) -> Option<T> {
        // SAFETY: `&self` here is a single shared borrow of the whole ECS,
        // not part of a Query's multi-storage access. No other unchecked
        // accessor is live concurrently with this one.
        unsafe { self.get_unchecked::<T>().get(entity) }.copied()
    }

    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        // SAFETY: `&mut self` here is the ordinary Rust borrow checker's
        // exclusivity guarantee. Nothing else can hold any reference into
        // this ECS while this &mut self borrow is alive, so calling the
        // otherwise-unchecked get_unchecked_mut is sound.
        unsafe { self.get_unchecked_mut::<T>().get_mut(entity) }
    }
}

pub struct Access {
    pub id: ComponentId,
    pub mutable: bool,
}

pub trait Query {
    type Item<'w>;
    const READ_ONLY: bool;
    fn accesses(out: &mut Vec<Access>);
    fn fetch(ecs: &ECS, entity: EntityId) -> Option<Self::Item<'_>>;
}

impl<T: Component> Query for &T {
    type Item<'w> = &'w T;
    const READ_ONLY: bool = true;
    fn accesses(out: &mut Vec<Access>) {
        out.push(Access {
            id: T::component_id(),
            mutable: false,
        });
    }
    fn fetch(ecs: &ECS, entity: EntityId) -> Option<Self::Item<'_>> {
        unsafe { ecs.get_unchecked::<T>().get(entity) }
    }
}

impl<T: Component> Query for &mut T {
    type Item<'w> = &'w mut T;
    const READ_ONLY: bool = false;
    fn accesses(out: &mut Vec<Access>) {
        out.push(Access {
            id: T::component_id(),
            mutable: true,
        });
    }
    fn fetch(ecs: &ECS, entity: EntityId) -> Option<Self::Item<'_>> {
        unsafe { ecs.get_unchecked_mut::<T>().get_mut(entity) }
    }
}

impl<T: Query> Query for Option<T> {
    type Item<'w> = Option<T::Item<'w>>;
    const READ_ONLY: bool = T::READ_ONLY;
    fn accesses(out: &mut Vec<Access>) {
        T::accesses(out);
    }
    fn fetch(ecs: &ECS, entity: EntityId) -> Option<Self::Item<'_>> {
        Some(T::fetch(ecs, entity))
    }
}

macro_rules! impl_query_for_tuple {
    ($($name:ident),*) => {
        impl<$($name: Query),*> Query for ($($name),*) {
            type Item<'w> = ($($name::Item<'w>),*);
            const READ_ONLY: bool = $($name::READ_ONLY)&&*;
            fn accesses(out: &mut Vec<Access>) {
                $($name::accesses(out));*
            }
            fn fetch(ecs: &ECS, entity: EntityId) -> Option<Self::Item<'_>> {
                Some(($($name::fetch(ecs, entity)?),*))
            }
        }
    };
}

impl_query_for_tuple!(A, B);
impl_query_for_tuple!(A, B, C);
impl_query_for_tuple!(A, B, C, D);
impl_query_for_tuple!(A, B, C, D, E);
impl_query_for_tuple!(A, B, C, D, E, F);
impl_query_for_tuple!(A, B, C, D, E, F, G);
impl_query_for_tuple!(A, B, C, D, E, F, G, H);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J);

fn check_no_conflicts(accesses: &[Access]) {
    for i in 0..accesses.len() {
        for j in (i + 1)..accesses.len() {
            let (a, b) = (&accesses[i], &accesses[j]);
            if a.id == b.id && (a.mutable || b.mutable) {
                panic!(
                    "query requests conflicting access to component {:?} (mutable: {} and {})",
                    a.id, a.mutable, b.mutable
                );
            }
        }
    }
}

pub struct QueryIter<'w, Q: Query> {
    ecs: &'w ECS,
    ids: std::vec::IntoIter<EntityId>,
    _marker: std::marker::PhantomData<Q>,
}

impl<'w, Q: Query> Iterator for QueryIter<'w, Q> {
    type Item = (EntityId, Q::Item<'w>);

    fn next(&mut self) -> Option<Self::Item> {
        for e in self.ids.by_ref() {
            if let Some(item) = Q::fetch(self.ecs, e) {
                return Some((e, item));
            }
        }
        None
    }
}

pub trait System {
    fn run(&mut self, world: &mut World, dt: f32);
}

impl<F: FnMut(&mut World, f32)> System for F {
    fn run(&mut self, world: &mut World, dt: f32) {
        self(world, dt)
    }
}

pub struct Scheduler {
    systems: Vec<Box<dyn System>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system(mut self, system: impl System + 'static) -> Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn run(&mut self, world: &mut World, dt: f32) {
        for system in &mut self.systems {
            system.run(world, dt);
        }
    }
}
