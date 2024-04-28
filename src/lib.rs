#![doc = include_str!("../README.md")]
use bevy_ecs::{component::Component, world::EntityWorldMut};
use bevy_ecs::world::EntityRef;
use serde::{Deserializer, Serializer};
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
pub mod entity;
mod filter;

pub use filter::EntityFilter;

#[allow(unused)]
use bevy_asset::Handle;
#[allow(unused)]
use bevy_hierarchy::Children;

#[allow(unused)]
pub use bevy_ecs::query::QueryData;

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
/// Can only be used in [`Serialize`] implementations.
pub fn with_world<T, S: Serializer>(f: impl FnOnce(&World) -> T) -> Result<T, S::Error> {
    if !WORLD.is_set() {
        Err(serde::ser::Error::custom("Cannot serialize outside the `save` scope"))
    } else {
        Ok(WORLD.with(f))
    }
    
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
pub fn with_world_mut<'de, T, S: Deserializer<'de>>(f: impl FnOnce(&mut World) -> T) -> Result<T, S::Error> {
    if !WORLD_MUT.is_set() {
        Err(serde::de::Error::custom("Cannot deserialize outside the `load` scope"))
    } else {
        Ok(WORLD_MUT.with(f))
    }
}


fn world_entity_scope<T, S: Serializer>(f: impl FnOnce(&World, Entity) -> T) -> Result<T, S::Error>{
    if !WORLD.is_set() {
        return Err(serde::ser::Error::custom("Cannot serialize outside the `save` scope"))
    }
    if !ENTITY.is_set() {
        return Err(serde::ser::Error::custom("No active entity found"))
    }
    Ok(WORLD.with(|w| {
        ENTITY.with(|e| f(w, *e))
    }))
}

fn world_entity_scope_mut<'de, T, S: Deserializer<'de>>(f: impl FnOnce(&mut World, Entity) -> T) -> Result<T, S::Error> {
    if !WORLD_MUT.is_set() {
        return Err(serde::de::Error::custom("Cannot deserialize outside the `load` scope"))
    }
    if !ENTITY.is_set() {
        return Err(serde::de::Error::custom("No active entity found"))
    }
    Ok(WORLD_MUT.with(|w| {
        ENTITY.with(|e| f(w, *e))
    }))
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

    /// A marker serialization object. This object should not hold data
    /// since data is stored in the world.
    type Object: Serialize + DeserializeOwned + ZstInit;

    /// If set and is a root node, use a query for serialization.
    /// Currently requires no children.
    const IS_QUERY: bool;
    /// If specified and `IS_QUERY` is set, 
    /// will use a query directly for serialization if is the root node.
    /// The user is responsible for making sure this roundtrips 
    /// since this does not affect deserialization.
    type Data: QueryData;
    /// Checks which entities the filter applies to.
    /// Entities that satisfies the filter **MUST** 
    /// satisfy the [`BevyObject`]'s layout.
    type Filter: EntityFilter;

    /// Obtain the root node to parent this component to if directly called.
    /// Default is `None`, which means no parent.
    #[allow(unused)]
    fn get_root(world: &mut World) -> Option<EntityWorldMut> {
        None
    }

    /// Name of the object, must be unique.
    fn name() -> &'static str;

    /// Initialize the serialization type.
    fn init() -> Self::Object {
        Self::Object::init()
    }

    /// Filter a [`EntityRef`].
    fn filter(entity: &EntityRef) -> bool {
        Self::Filter::filter(entity)
    }
}

impl<T> BevyObject for T where T: Component + Serialize + DeserializeOwned + TypePath {
    const IS_QUERY: bool = true;
    type Object = SerializeComponent<T>;

    type Data = &'static T;
    type Filter = With<T>;

    fn name() -> &'static str {
        T::short_type_path()
    }
}

/// Make a type usable in in the [`bind_object!`] macro.
pub trait BindProject {
    type To: Serialize + DeserializeOwned + ZstInit;
}

/// Make a type usable in in the [`bind_query!`] macro.
pub trait BindProjectQuery {
    type Data: QueryData;
}

impl<T> BindProject for T where T: BevyObject {
    type To = T::Object;
}

impl<T> BindProjectQuery for T where T: BevyObject {
    type Data = T::Data;
}
