#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::type_complexity)]
#![allow(clippy::collapsible_else_if)]
use bevy::ecs::component::Component;
use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::ecs::world::EntityRef;
mod extractors;
pub use extractors::*;
mod children;
pub use children::{Child, ChildVec, ChildrenLike};
mod batch;
mod childmap;
pub use childmap::{ChildMap, ChildMapLike};
mod extensions;
mod root;
pub use batch::{BatchSerialization, Join, SerializeWorld};
pub use extensions::{InWorld, SerializeLens, WorldExtension};
pub use root::RootObject;
mod adjacent;
pub use adjacent::{Adjacent, SerializeAdjacent};
pub mod asset;
mod filter;
pub mod interning;
pub mod typetagged;
mod util;
pub use filter::EntityFilter;
pub use util::*;
#[cfg(any(feature = "linkme", doc))]
#[cfg_attr(docsrs, doc(cfg(feature = "linkme")))]
pub mod linking;

#[allow(unused)]
use bevy::asset::Handle;
#[allow(unused)]
use bevy::ecs::hierarchy::Children;

#[doc(hidden)]
pub use bevy::ecs::{
    entity::Entity,
    query::With,
    world::{EntityWorldMut, World},
};
#[doc(hidden)]
pub use bevy::reflect::TypePath;
#[doc(hidden)]
pub use serde;
#[allow(unused)]
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use bevy_serde_lens_core::{DeUtils, SerUtils};
#[cfg(feature = "derive")]
pub use bevy_serde_lens_derive::BevyObject;

/// Equivalent to [`Default`], indicates the type should be a marker ZST, not a concrete type.
///
/// Due to the role of [`Default`] in `#[serde(default)]` and `#[serde(skip)]`,
/// `Default` is not appropriate on certain types.
pub trait ZstInit: Sized {
    fn init() -> Self;
}

#[doc(hidden)]
pub type Item<'t, T> = <<<T as BevyObject>::Data as QueryData>::ReadOnly as QueryData>::Item<'t, 't>;
#[doc(hidden)]
pub type BindItem<'t, T> =
    <<<T as BindProjectQuery>::Data as QueryData>::ReadOnly as QueryData>::Item<'t, 't>;

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

    /// Convert `Data` to a serializable, must specify if `IS_QUERY`.
    #[allow(unused_variables)]
    fn into_ser(query_data: Item<'_, Self>) -> impl Serialize {}
}

impl<T> BevyObject for T
where
    T: Component + Serialize + DeserializeOwned + TypePath,
{
    const IS_QUERY: bool = true;
    type Object = SerializeComponent<T>;

    type Data = &'static T;
    type Filter = With<T>;

    fn name() -> &'static str {
        T::short_type_path()
    }

    fn into_ser(query_data: Item<'_, Self>) -> impl Serialize {
        query_data
    }
}

/// Make a type usable in the [`BevyObject`] macro.
pub trait BindProject {
    type To: ZstInit;
    type Filter: QueryFilter;
}

/// Make a type usable in the [`BevyObject`] macro in `query` mode.
pub trait BindProjectQuery {
    type Data: QueryData;
}

impl<T> BindProject for T
where
    T: BevyObject,
{
    type To = T::Object;
    /// Optionally used in the macro if `Filter` is not specified.
    type Filter = T::Filter;
}

impl<T> BindProjectQuery for T
where
    T: BevyObject,
{
    type Data = T::Data;
}

/// Batches multiple [`SerializeWorld`] types to be serialized together as a map.
///
/// This macro generates a `type` that can be used on `World::save` and `World::load`.
///
/// # Example
///
/// ```
/// type SerializeItems = serialize_group!(Potion, Weapon, Armor);
/// ```
#[macro_export]
macro_rules! batch {
    ($($tt:tt)*) => {
        $crate::batch_inner!($($tt)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! batch_inner {
    ($ty: ty $(,)?) => {
        $ty
    };
    ($a: ty, $b: ty $(,)?) => {
        $crate::Join<$a, $b>
    };
    ($first: ty $(,$ty: ty)* $(,)?) => {
        $crate::Join<$first, $crate::batch_inner!($($ty),*)>
    };
}

#[doc(hidden)]
pub use std::format;
