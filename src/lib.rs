#![doc = include_str!("../README.md")]
use bevy_ecs::{component::Component, world::EntityWorldMut};
use bevy_ecs::world::EntityRef;
use serde::{de::DeserializeOwned, Serialize};
mod extractors;
pub use extractors::*;
mod save_load;
mod batch;
pub use batch::{Join, BatchSerialization, SerializeWorld};
pub use save_load::{WorldExtension, SerializeLens, DeserializeLens, ScopedDeserializeLens};
mod macros;
pub mod interning;
pub mod asset;
pub mod typetagged;
// pub mod typetagged;
// pub mod entity;
mod filter;

pub use filter::EntityFilter;

#[allow(unused)]
use bevy_asset::Handle;
#[allow(unused)]
use bevy_hierarchy::Children;

#[doc(hidden)]
pub use bevy_ecs::{world::World, entity::Entity, query::With};
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use paste::paste;
#[doc(hidden)]
pub use bevy_reflect::TypePath;

scoped_tls_hkt::scoped_thread_local!( 
    static ENTITY: Entity
);

scoped_tls_hkt::scoped_thread_local!( 
    static WORLD: World
);

scoped_tls_hkt::scoped_thread_local!( 
    static mut WORLD_MUT: World
);

/// Run a function on a read only reference to [`World`].
/// 
/// Can only be used in [`Serialize`](serde::Serialize) implementations.
pub fn with_world<T>(f: impl FnOnce(&World) -> T) -> T {
    WORLD.with(f)
}

/// Run a function on a mutable only reference to [`World`].
/// 
/// Can only be used in [`Deserialize`](serde::Deserialize) implementations.
/// 
/// # Panics
/// 
/// If used in a nested manner, as that is a violation to rust's aliasing rule.
/// 
/// ```
/// with_world_mut(|| {
///     // panics here
///     with_world_mut(|| {
///         ..
///     })
/// })
/// ```
pub fn with_world_mut<T>(f: impl FnOnce(&mut World) -> T) -> T {
    WORLD_MUT.with(f)
}


fn world_entity_scope<T>(f: impl FnOnce(&World, Entity) -> T) -> T{
    if !WORLD.is_set() {
        panic!("Cannot serialize outside the `save` scope")
    }
    if !ENTITY.is_set() {
        panic!("No active entity found")
    }
    WORLD.with(|w| {
        ENTITY.with(|e| f(w, *e))
    })
}

fn world_entity_scope_mut<T>(f: impl FnOnce(&mut World, Entity) -> T) -> T{
    if !WORLD_MUT.is_set() {
        panic!("Cannot deserialize outside the `load` scope")
    }
    if !ENTITY.is_set() {
        panic!("No active entity found")
    }
    WORLD_MUT.with(|w| {
        ENTITY.with(|e| f(w, *e))
    })
}

/// Equivalent to [`Default`], indicates the type should be a marker ZST, not a concrete type.
/// 
/// Due to the role of [`Default`] in `#[serde(skip)]`,
/// `Default` should not be implemented on certain types.
pub trait ZstInit: Sized {
    fn init() -> Self;
}

/// Associate a [`BevyObject`] to a [`EntityFilter`], usually a component as `With<Component>`.
///
/// This means `world.save::<T>()` will try to serialize all entities that satisfies the filter.
pub trait BevyObject {
    type Object: Serialize + DeserializeOwned + ZstInit;

    type Filter: EntityFilter;

    /// Obtain the root node to parent this component to if directly called.
    /// Default is `None`, which means no parent.
    #[allow(unused)]
    fn get_root(world: &mut World) -> Option<EntityWorldMut> {
        None
    }

    /// Name of the object, must be unique.
    fn name() -> &'static str;

    fn init() -> Self::Object {
        Self::Object::init()
    }

    fn filter(entity: &EntityRef) -> bool {
        Self::Filter::filter(entity)
    }
}

impl<T> BevyObject for T where T: Component + Serialize + DeserializeOwned + TypePath {
    type Object = SerializeComponent<T>;

    type Filter = With<T>;

    fn name() -> &'static str {
        T::short_type_path()
    }
}

/// Make a type usable in in the [`bind_object!`] macro.
pub trait BindProject {
    type To: Serialize + DeserializeOwned + ZstInit;
}

impl<T> BindProject for T where T: BevyObject {
    type To = T::Object;
}