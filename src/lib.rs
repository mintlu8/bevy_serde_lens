#![doc = include_str!("../README.md")]

use std::cell::Cell;

use bevy_ecs::component::Component;
use bevy_ecs::world::EntityRef;
use bevy_reflect::TypePath;
use extractors::SerializeComponent;
use sealed::Sealed;
use serde::{de::DeserializeOwned, Serialize};
mod extractors;
pub use extractors::{Object, DefaultInit, Maybe, Child};
mod save_load;
//pub use save_load::{WorldExtension, Join, SaveLoad, BindResource};
mod macros;
// pub mod typetagged;
// pub mod asset;
// pub mod interning;
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

fn entity_missing<T, S: serde::Serializer>(entity: Entity) -> Result<T, S::Error> {
    Err(serde::ser::Error::custom(format!("Entity {entity:?} missing")))
}

fn entity_missing_de<'de, T, S: serde::Deserializer<'de>>(entity: Entity) -> Result<T, S::Error> {
    Err(serde::de::Error::custom(format!("Entity {entity:?} missing")))
}

fn not_found<T, S: serde::Serializer>() -> Result<T, S::Error> {
    Err(serde::ser::Error::custom("Bevy object not found."))
}

thread_local! {
    //static WORLD: Cell<*mut World> = const { Cell::new(std::ptr::null_mut()) };
    static ERR: Cell<bool> = Cell::new(false);
}

scoped_tls_hkt::scoped_thread_local!( 
    static ENTITY: Entity
);

scoped_tls_hkt::scoped_thread_local!( 
    static WORLD: World
);

scoped_tls_hkt::scoped_thread_local!( 
    static mut WORLD_MUT: World
);


fn world_entity_scope<T>(f: impl FnOnce(&World, Entity) -> T) -> T{
    WORLD.with(|w| {
        ENTITY.with(|e| f(w, *e))
    })
}

fn world_entity_scope_mut<T>(f: impl FnOnce(&mut World, Entity) -> T) -> T{
    WORLD_MUT.with(|w| {
        ENTITY.with(|e| f(w, *e))
    })
}


pub trait ZstInit: Sized {
    fn init() -> Self;
}

mod sealed {
    pub trait Sealed {}
}


impl Sealed for () {}
pub struct ComponentToken;

impl Sealed for ComponentToken {}
pub struct ResourceToken;

impl Sealed for ResourceToken {}
pub struct NonSendResourceToken;

impl Sealed for NonSendResourceToken {}

/// Associate a [`BevyObject`] to a [`EntityFilter`], usually a component as `With<Component>`.
///
/// This means `world.save::<T>()` will try to serialize all entities that satisfies the filter.
pub trait BevyObject {
    type Object: Serialize + DeserializeOwned + ZstInit;

    type Filter: EntityFilter;

    /// Obtain the root node to parent this component to if directly called.
    /// Default is `None`, which means no parent.
    #[allow(unused)]
    fn get_root(world: &mut World) -> Option<Entity> {
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

    type Filter = ();

    fn name() -> &'static str {
        T::short_type_path()
    }
}
