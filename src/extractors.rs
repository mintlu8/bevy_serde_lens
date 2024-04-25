use std::{any::type_name, marker::PhantomData};

use bevy_ecs::{system::Resource, world::FromWorld};
use bevy_hierarchy::{BuildWorldChildren, Children};
use serde::{de::{DeserializeOwned, SeqAccess, Visitor}, Deserialize, Deserializer, Serialize, Serializer};
use crate::{world_entity_scope, world_entity_scope_mut, BevyObject, BindProject, ZstInit, ENTITY, WORLD, WORLD_MUT};

#[allow(unused)]
use bevy_ecs::component::Component;

/// Extractor that allows a [`BevyObject`] to be missing.
///
/// The underlying data structure is `Option`, 
/// so you can use `#[serde(skip_deserializing_if("Option::is_none"))]`.
pub struct Maybe<T>(PhantomData<T>);

impl<T: BevyObject> ZstInit for Maybe<T> {
    fn init() -> Self { Self(PhantomData) }
}

impl<T: BevyObject> Default for Maybe<T> {
    fn default() -> Self { Self(PhantomData) }
}


impl<T: BevyObject> BindProject for Maybe<T> {
    type To = Self;
}

impl<T: BevyObject> Serialize for Maybe<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        WORLD.with(|world| {
            ENTITY.with(|entity| {
                match world.get_entity(*entity) {
                    Some(entity_ref) => if T::filter(&entity_ref) {
                        Some(T::init())
                    } else {
                        None
                    },
                    None => None,
                }
            })
        }).serialize(serializer)
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Maybe<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Option<T::Object>>::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

impl<T: BevyObject> ZstInit for Maybe<Child<T>> {
    fn init() -> Self { Self(PhantomData) }
}

impl<T: BevyObject> Default for Maybe<Child<T>> {
    fn default() -> Self { Self(PhantomData) }
}

impl<T: BevyObject> BindProject for Maybe<Child<T>> {
    type To = Self;
}

impl<T: BevyObject> Serialize for Maybe<Child<T>> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return Err(serde::ser::Error::custom(format!(
                        "Entity missing {entity:?}."
                    )));
                };
                let Some(children) = entity.get::<Children>() else {
                    return Err(serde::ser::Error::custom(format!(
                        "No children found for {}.", type_name::<T>()
                    )));
                };
                for entity in children {
                    let Some(entity) = world.get_entity(*entity) else {continue};
                    if T::filter(&entity) {
                        return ENTITY.set(&entity.id(), || {
                            Some(T::init()).serialize(serializer)
                        })
                    }
                }
                None::<T::Object>.serialize(serializer)
            }
        )?
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Maybe<Child<T>> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Option<Child<T>>>::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

/// Convert a [`Default`] or [`FromWorld`] component to [`BevyObject`] using
/// default initialization. 
/// 
/// Use `#[serde(skip)]` to skip serializing this component completely.
pub struct DefaultInit<T>(PhantomData<T>);

impl<T> ZstInit for DefaultInit<T> {
    fn init() -> Self { Self(PhantomData) }
}

impl<T: Component + Serialize + DeserializeOwned + Default> BindProject for DefaultInit<T> {
    type To = Self;
}

impl<T: FromWorld> Serialize for DefaultInit<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        ().serialize(serializer)
    }
}

impl<'de, T: FromWorld> Deserialize<'de> for DefaultInit<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        <()>::deserialize(deserializer)?;
        WORLD_MUT.with(|w|T::from_world(w));
        Ok(Self(PhantomData))
    }
}

pub struct Root<T>(PhantomData<T>);

impl<T> ZstInit for Root<T> {
    fn init() -> Self { Self(PhantomData) }
}

impl<'de, T: BevyObject> Deserialize<'de> for Root<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_seq(Root(PhantomData))
    }
}

impl<'de, T: BevyObject> Visitor<'de> for Root<T> {
    type Value = Root<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>, {
        loop {
            let entity = WORLD_MUT.with(|world| {
                let entity = world.spawn_empty().id();
                if let Some(mut root) = T::get_root(world) {
                    root.add_child(entity);
                }
                entity
            });
            if ENTITY.set(&entity, ||seq.next_element::<T::Object>())?.is_none() {
                WORLD_MUT.with(|world| world.despawn(entity));
                break
            }
        }
        Ok(Root(PhantomData))
    }
}

pub struct SerializeComponent<T>(PhantomData<T>);

impl<T> ZstInit for SerializeComponent<T> {
    fn init() -> Self { Self(PhantomData) }
}

impl<T: Component + Serialize + DeserializeOwned> BindProject for SerializeComponent<T> {
    type To = Self;
}

impl<T: Component + Serialize> Serialize for SerializeComponent<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return Err(serde::ser::Error::custom(format!(
                        "Entity missing: {entity:?}."
                    )));
                };
                let Some(component) = entity.get::<T>() else {
                    return Err(serde::ser::Error::custom(format!(
                        "Component missing: {}.", std::any::type_name::<T>()
                    )));
                };
                component.serialize(serializer)
            }
        )?
    }
}

impl<'de, T: Component + Deserialize<'de>> Deserialize<'de> for SerializeComponent<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let component = T::deserialize(deserializer)?;
        world_entity_scope_mut::<_, D>(
            |world, entity| {
                let Some(mut entity) = world.get_entity_mut(entity) else {
                    return Err(serde::de::Error::custom(format!(
                        "Entity missing {entity:?}."
                    )));
                };
                entity.insert(component);
                Ok(Self(PhantomData))
            }
        )?
    }
}

pub struct SerializeResource<T>(PhantomData<T>);

impl<T> ZstInit for SerializeResource<T> {
    fn init() -> Self { Self(PhantomData) }
}

impl<T: Resource + Serialize> Serialize for SerializeResource<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        WORLD.with(
            |world| {
                let Some(resource) = world.get_resource::<T>() else {
                    return Err(serde::ser::Error::custom(format!(
                        "Resource missing {}.", std::any::type_name::<T>()
                    )));
                };
                resource.serialize(serializer)
            }
        )
    }
}

impl<'de, T: Resource + Deserialize<'de>> Deserialize<'de> for SerializeResource<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let resource = T::deserialize(deserializer)?;
        WORLD_MUT.with(|world| world.insert_resource(resource) );
        Ok(Self(PhantomData))
    }
}

pub struct SerializeNonSend<T>(PhantomData<T>);

impl<T> ZstInit for SerializeNonSend<T> {
    fn init() -> Self { Self(PhantomData) }
}

impl<T: Serialize + 'static> Serialize for SerializeNonSend<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        WORLD.with(
            |world| {
                let Some(resource) = world.get_non_send_resource::<T>() else {
                    return Err(serde::ser::Error::custom(format!(
                        "Non-send resource missing {}.", std::any::type_name::<T>()
                    )));
                };
                resource.serialize(serializer)
            }
        )
    }
}

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de> for SerializeNonSend<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let resource = T::deserialize(deserializer)?;
        WORLD_MUT.with(|world| world.insert_non_send_resource(resource));
        Ok(Self(PhantomData))
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// instead of the entity itself. 
pub struct Child<T>(PhantomData<T>);

impl<T> ZstInit for Child<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject> BindProject for Child<T> {
    type To = Self;
}


impl<T: BevyObject> Serialize for Child<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        world_entity_scope::<_, S>(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return Err(serde::ser::Error::custom(format!(
                        "Entity missing {entity:?}."
                    )));
                };
                let Some(children) = entity.get::<Children>() else {
                    return Err(serde::ser::Error::custom(format!(
                        "No children found for {}.", type_name::<T>()
                    )));
                };
                for entity in children {
                    let Some(entity) = world.get_entity(*entity) else {continue};
                    if T::filter(&entity) {
                        return ENTITY.set(&entity.id(), || {
                            T::init().serialize(serializer)
                        })
                    }
                }
                Err(serde::ser::Error::custom(format!(
                    "No valid children found for {}.", type_name::<T>()
                )))
            }
        )?
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Child<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        world_entity_scope_mut::<_, D>(|world, entity| {
            let new_child = world.spawn_empty().id();
            ENTITY.set(&new_child, || {
                <T::Object>::deserialize(deserializer)
            })?;
            world.entity_mut(entity).add_child(new_child);
            Ok(Child(PhantomData))
        })?
    }
}

/// Extractor for multiple [`BevyObject`]s in [`Children`]
/// instead of the entity itself. This serializes children like a `Vec`.
pub struct ChildVec<T>(PhantomData<T>);

impl<T> ZstInit for ChildVec<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

/// This is allowed since `0` is a valid number of children.
impl<T> Default for ChildVec<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject> Serialize for ChildVec<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeSeq;
        world_entity_scope::<_, S>(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return Err(serde::ser::Error::custom(format!(
                        "Entity missing {entity:?}."
                    )));
                };
                let children = match entity.get::<Children>() {
                    Some(children) => children.as_ref(),
                    None => &[],
                };
                let count = children.iter()
                    .filter_map(|e| world.get_entity(*e))
                    .filter(T::filter)
                    .count();
                let mut seq = serializer.serialize_seq(Some(count))?;
                for entity in children.iter().filter_map(|e| world.get_entity(*e)).filter(T::filter) {
                    ENTITY.set(&entity.id(), ||{
                        seq.serialize_element(&T::init())
                    })?
                }
                seq.end()
            }
        )?
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for ChildVec<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_seq(ChildVec(PhantomData))
    }
}

impl<'de, T: BevyObject> Visitor<'de>  for ChildVec<T> {
    type Value = ChildVec<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        while seq.next_element::<Child<T>>()?.is_some() {}
        Ok(ChildVec(PhantomData))
    }
}

impl<T: BevyObject> BindProject for ChildVec<T> {
    type To = Self;
}