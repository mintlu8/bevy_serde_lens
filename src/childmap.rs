use bevy_ecs::{component::Component, entity::Entity, world::EntityWorldMut};
use bevy_serde_lens_core::{DeUtils, ScopeUtils, SerUtils};
use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Serialize,
};
use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use crate::{root::RootObject, BevyObject, BindProject, ZstInit};

/// Types that references one or many entities with a serializable key.
pub trait ChildMapLike: Component + Sized {
    type Key: Serialize + DeserializeOwned + 'static;
    /// Iterate over its children.
    ///
    /// If only one child exists, use [`std::iter::once`].
    fn iter_children(&self) -> impl Iterator<Item = (&Self::Key, Entity)>;
    /// Function that add child to parent, like [`EntityWorldMut::add_related`].
    fn add_child(parent: EntityWorldMut, key: Self::Key, child: Entity)
        -> Result<(), impl Display>;
}

/// Extractor for multiple [`BevyObject`]s in [`Children`]
/// or entities referenced by a custom [`ChildrenLike`] type.
///
/// This serializes children like a `Vec`.
pub struct ChildMap<T, C>(PhantomData<(T, C)>);

impl<T, C> Debug for ChildMap<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildMap").finish()
    }
}

impl<T, C> ZstInit for ChildMap<T, C> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

/// This is allowed since `0` is a valid number of children.
impl<T, C> Default for ChildMap<T, C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject, C: ChildMapLike> Serialize for ChildMap<T, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let entity = SerUtils::current_entity::<S>()?;
        SerUtils::with_world::<S, _>(|world| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            let Some(children) = entity.get::<C>() else {
                return serializer.serialize_map(Some(0))?.end();
            };
            let count = children
                .iter_children()
                .filter_map(|(_, e)| world.get_entity(e).ok())
                .filter(T::filter)
                .count();
            let mut seq = serializer.serialize_map(Some(count))?;
            for (key, entity) in children.iter_children().filter_map(|(key, entity)| {
                if let Ok(entity) = world.get_entity(entity) {
                    if T::filter(&entity) {
                        return Some((key, entity));
                    }
                }
                None
            }) {
                ScopeUtils::current_entity_scope(entity.id(), || {
                    seq.serialize_entry(&key, &T::init())
                })?;
            }
            seq.end()
        })?
    }
}

impl<'de, T: BevyObject, C: ChildMapLike> Deserialize<'de> for ChildMap<T, C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChildMap::<T, C>(PhantomData))
    }
}

impl<'de, T: BevyObject, C: ChildMapLike> Visitor<'de> for ChildMap<T, C> {
    type Value = ChildMap<T, C>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_map<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some((key, child)) = seq.next_entry::<C::Key, RootObject<T>>()? {
            DeUtils::with_entity_mut_err::<A::Error, _>(|parent| {
                C::add_child(parent, key, child.get()).map_err(serde::de::Error::custom)
            })??;
        }
        Ok(ChildMap(PhantomData))
    }
}

impl<T: BevyObject, C: ChildMapLike> BindProject for ChildMap<T, C> {
    type To = Self;
    type Filter = ();
}
