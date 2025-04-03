use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::relationship::RelationshipTarget;
use bevy_ecs::world::EntityWorldMut;
use bevy_ecs::hierarchy::Children;
use bevy_serde_lens_core::private::entity_scope;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display};
use std::{any::type_name, marker::PhantomData};

use crate::{world_entity_scope, world_entity_scope_mut, BevyObject, BindProject, Maybe, ZstInit};

/// Types that references one or many entities similar to [`Children`].
pub trait ChildrenLike: Component + Sized {
    /// Iterate over its children.
    ///
    /// If only one child exists, use [`std::iter::once`].
    fn iter_children(&self) -> impl Iterator<Item = Entity>;
    /// Function that add child to parent, like [`BuildChildren::add_child`].
    ///
    /// Keep in mind this is responsible for initializing components as well.
    fn add_child(parent: EntityWorldMut, child: Entity) -> Result<(), impl Display>;
}

impl<T> ChildrenLike for T where T: RelationshipTarget {
    fn iter_children(&self) -> impl Iterator<Item = Entity> {
        T::iter(self)
    }

    fn add_child(mut parent: EntityWorldMut, child: Entity) -> Result<(), impl Display> {
        parent.add_related::<T::Relationship>(&[child]);
        Ok::<(), &'static str>(())
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// or entities referenced by a custom [`ChildrenLike`] type.
pub struct Child<T, C = Children>(PhantomData<(T, C)>);

impl<T, C> ZstInit for Child<T, C> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T, C> Debug for Child<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Child").finish()
    }
}

impl<T: BevyObject, C> BindProject for Child<T, C> {
    type To = Self;
    type Filter = ();
}

impl<T: BevyObject, C: ChildrenLike> Serialize for Child<T, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            let Some(children) = entity.get::<C>() else {
                return Err(serde::ser::Error::custom(format!(
                    "No children found for {}.",
                    type_name::<T>()
                )));
            };
            for entity in children.iter_children() {
                let Ok(entity) = world.get_entity(entity) else {
                    continue;
                };
                if T::filter(&entity) {
                    return entity_scope(entity.id(), || T::init().serialize(serializer));
                }
            }
            Err(serde::ser::Error::custom(format!(
                "No valid children found for {}.",
                type_name::<T>()
            )))
        })?
    }
}

impl<'de, T: BevyObject, C: ChildrenLike> Deserialize<'de> for Child<T, C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let new_child = world_entity_scope_mut::<_, D>(|world, entity| {
            let child = world.spawn_empty().id();
            C::add_child(world.entity_mut(entity), child).map_err(serde::de::Error::custom)?;
            Ok(child)
        })??;
        entity_scope(new_child, || <T::Object>::deserialize(deserializer))
            .map_err(serde::de::Error::custom)?;
        Ok(Child(PhantomData))
    }
}

impl<T, C> Default for Maybe<Child<T, C>> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, C> BindProject for Maybe<Child<T, C>> {
    type To = Self;
    type Filter = ();
}

impl<T: BevyObject, C: ChildrenLike> Serialize for Maybe<Child<T, C>> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            let Some(children) = entity.get::<C>() else {
                return None::<T::Object>.serialize(serializer);
            };
            for entity in children.iter_children() {
                let Ok(entity) = world.get_entity(entity) else {
                    continue;
                };
                if T::filter(&entity) {
                    return entity_scope(entity.id(), || Some(T::init()).serialize(serializer))
                        .map_err(serde::ser::Error::custom);
                }
            }
            None::<T::Object>.serialize(serializer)
        })?
    }
}

impl<'de, T: BevyObject, C: ChildrenLike> Deserialize<'de> for Maybe<Child<T, C>> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Option<Child<T, C>>>::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

/// Extractor for multiple [`BevyObject`]s in [`Children`]
/// or entities referenced by a custom [`ChildrenLike`] type.
///
/// This serializes children like a `Vec`.
pub struct ChildVec<T, C = Children>(PhantomData<(T, C)>);

impl<T, C> Debug for ChildVec<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildVec").finish()
    }
}

impl<T, C> ZstInit for ChildVec<T, C> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

/// This is allowed since `0` is a valid number of children.
impl<T, C> Default for ChildVec<T, C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject, C: ChildrenLike> Serialize for ChildVec<T, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            let Some(children) = entity.get::<C>() else {
                return serializer.serialize_seq(Some(0))?.end();
            };
            let count = children
                .iter_children()
                .filter_map(|e| world.get_entity(e).ok())
                .filter(T::filter)
                .count();
            let mut seq = serializer.serialize_seq(Some(count))?;
            for entity in children
                .iter_children()
                .filter_map(|e| world.get_entity(e).ok())
                .filter(T::filter)
            {
                entity_scope(entity.id(), || seq.serialize_element(&T::init()))?;
            }
            seq.end()
        })?
    }
}

impl<'de, T: BevyObject, C: ChildrenLike> Deserialize<'de> for ChildVec<T, C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChildVec::<T, C>(PhantomData))
    }
}

impl<'de, T: BevyObject, C: ChildrenLike> Visitor<'de> for ChildVec<T, C> {
    type Value = ChildVec<T, C>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<Child<T, C>>()?.is_some() {}
        Ok(ChildVec(PhantomData))
    }
}

impl<T: BevyObject, C: ChildrenLike> BindProject for ChildVec<T, C> {
    type To = Self;
    type Filter = ();
}
