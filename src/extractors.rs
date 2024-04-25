use std::marker::PhantomData;

use bevy_ecs::{system::Resource, world::FromWorld};
use bevy_hierarchy::{BuildWorldChildren, Children};
use serde::{de::{SeqAccess, Visitor}, Deserialize, Serialize, Serializer};
use crate::{entity_missing, entity_missing_de, not_found, world_entity_scope, world_entity_scope_mut, BevyObject, ZstInit, ENTITY, WORLD, WORLD_MUT};

#[allow(unused)]
use bevy_ecs::component::Component;

/// Extractor for casting a [`BindBevyObject`] to its bound [`BevyObject`].
pub type Object<T> = <T as BevyObject>::Object;

/// Extractor that allows a [`BevyObject`] to be missing.
///
/// The underlying data structure is `Option`, 
/// so you can use `#[serde(skip_deserializing_if("Option::is_none"))]`.
pub struct Maybe<T>(PhantomData<T>);

/// Convert a [`Default`] or [`FromWorld`] component to [`BevyObject`] using
/// default initialization. 
/// 
/// Use `#[serde(skip)]` to skip serializing this component completely.
pub struct DefaultInit<T>(PhantomData<T>);

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
            let entity = WORLD_MUT.with(|world| {world.spawn_empty().id()});
            if seq.next_element::<T::Object>()?.is_none() {
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

impl<T: Component + Serialize> Serialize for SerializeComponent<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return entity_missing::<_, S>(entity);
                };
                let Some(component) = entity.get::<T>() else {
                    return not_found::<_, S>();
                };
                component.serialize(serializer)
            }
        )
    }
}

impl<'de, T: Component + Deserialize<'de>> Deserialize<'de> for SerializeComponent<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let component = T::deserialize(deserializer)?;
        world_entity_scope_mut(
            |world, entity| {
                let Some(mut entity) = world.get_entity_mut(entity) else {
                    return entity_missing_de::<_, D>(entity);
                };
                entity.insert(component);
                Ok(Self(PhantomData))
            }
        )
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
                    return not_found::<_, S>();
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
                    return not_found::<_, S>();
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
///
/// This will iterate through all children
/// to validate uniqueness. [`ChildUnchecked`] is a non-checking
/// alternative. Alternatively use [`ChildVec`] for a list of objects.
///
/// # Errors
///
/// When more than one item is found.
pub struct Child<T>(PhantomData<T>);

impl<T: BevyObject> Serialize for Child<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        world_entity_scope(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return entity_missing::<_, S>(entity);
                };
                let Some(children) = entity.get::<Children>() else {
                    return not_found::<_, S>();
                };
                for entity in children {
                    let Some(entity) = world.get_entity(*entity) else {continue};
                    if T::filter(&entity) {
                        return ENTITY.set(&entity.id(), || {
                            T::init().serialize(serializer)
                        })
                    }
                }
                not_found::<_, S>()
            }
        )
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Child<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        world_entity_scope_mut(|world, entity| {
            let new_child = world.spawn_empty().id();
            ENTITY.set(&new_child, || {
                <T::Object>::deserialize(deserializer)
            })?;
            world.entity_mut(entity).add_child(new_child);
            Ok(Child(PhantomData))
        })
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// instead of the entity itself. 
///
/// This will iterate through all children
/// to validate uniqueness. [`ChildUnchecked`] is a non-checking
/// alternative. Alternatively use [`ChildVec`] for a list of objects.
///
/// # Errors
///
/// When more than one item is found.
pub struct ChildVec<T>(PhantomData<T>);

impl<T: BevyObject> Serialize for ChildVec<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeSeq;
        world_entity_scope(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return entity_missing::<_, S>(entity);
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
        )
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

// /// Extractor for matching [`BevyObject`]s on a [`Children`].
// /// 
// /// Unlike [`ChildVec`] this tries to present a map.
// ///
// /// The underlying data structure is a [`Map`], 
// /// so you can use `#[serde(skip_serializing_if("Map::is_empty"))]`.
// pub struct ChildMap<K, V>(PhantomData<(K, V)>);

// impl<K, V> BevyObject for ChildMap<K, V> where K: BevyObject, V: BevyObject {
//     type Ser<'t> = Map<K::Ser<'t>, V::Ser<'t>> where K: 't, V: 't;
//     type De<'de> = Map<K::De<'de>, V::De<'de>>;

//     fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
//         let Some(children) = world.entity_ok(entity)?.get::<Children>() else {
//             return Ok(Some(Map::new()));
//         };
//         children.iter()
//             .filter_map(|entity|Some ((
//                 K::to_ser(world, *entity).transpose()?, 
//                 V::to_ser(world, *entity), 
//             )))
//             .map(|(key, value)| {Ok((
//                 key?,
//                 value?.ok_or_else(||Error::KeyNoValue { 
//                     key: type_name::<K>(), 
//                     value: type_name::<V>(), 
//                 })?
//             ))})
//             .collect::<Result<Map<_, _>, _>>()
//             .map(Some)
//     }

//     fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
//         for item in de {
//             let entity = world.spawn_empty().id();
//             K::from_de(world, entity, item.0)?;
//             V::from_de(world, entity, item.1)?;
//             world.entity_mut(parent).add_child(entity);
//         }
//         Ok(())
//     }
// }
