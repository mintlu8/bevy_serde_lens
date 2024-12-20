use std::{any::type_name, fmt::Debug, marker::PhantomData};

use crate::{
    entity_scope, world_entity_scope, world_entity_scope_mut, BevyObject, BindProject,
    BindProjectQuery, ZstInit,
};
use bevy_ecs::{
    entity::Entity,
    query::{QueryFilter, With},
    system::Resource,
    world::{FromWorld, World},
};
use bevy_hierarchy::{BuildChildren, Children, DespawnRecursiveExt};
use bevy_serde_lens_core::{with_world, with_world_mut};
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

#[allow(unused)]
use bevy_ecs::component::Component;

/// Extractor that allows a [`BevyObject`] to be missing.
///
/// `#[serde(default)]` can be used to make this optional
/// if used in self describing formats.
pub struct Maybe<T>(PhantomData<T>);

impl<T> Debug for Maybe<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Maybe").finish()
    }
}

impl<T> ZstInit for Maybe<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject> Default for Maybe<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject> BindProject for Maybe<T> {
    type To = Self;
    type Filter = ();
}

impl<T: BevyObject> BindProjectQuery for Maybe<T> {
    type Data = Option<T::Data>;
}

impl<T: BevyObject> Serialize for Maybe<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| match world.get_entity(entity) {
            Ok(entity_ref) => {
                if T::filter(&entity_ref) {
                    Some(T::init())
                } else {
                    None
                }
            }
            Err(_) => None,
        })?
        .serialize(serializer)
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Maybe<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Option<T::Object>>::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

impl<T: BevyObject> Default for Maybe<Child<T>> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: BevyObject> BindProject for Maybe<Child<T>> {
    type To = Self;
    type Filter = ();
}

impl<T: BevyObject> Serialize for Maybe<Child<T>> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            let Some(children) = entity.get::<Children>() else {
                return Err(serde::ser::Error::custom(format!(
                    "No children found for {}.",
                    type_name::<T>()
                )));
            };
            for entity in children {
                let Ok(entity) = world.get_entity(*entity) else {
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

impl<T> Debug for DefaultInit<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultInit").finish()
    }
}

impl<T> ZstInit for DefaultInit<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

type DummyDeserializer = serde::de::value::BoolDeserializer<serde::de::value::Error>;

/// Here to make `#[serde(default)]` work.
impl<T: Component + FromWorld> Default for DefaultInit<T> {
    fn default() -> Self {
        let _ = world_entity_scope_mut::<_, DummyDeserializer>(|world, entity| {
            let item = T::from_world(world);
            let Ok(mut entity) = world.get_entity_mut(entity) else {
                return;
            };
            entity.insert(item);
        });
        Self(PhantomData)
    }
}

impl<T: Component + FromWorld> BindProject for DefaultInit<T> {
    type To = Self;
    type Filter = With<T>;
}

impl<T: Component + FromWorld> BindProjectQuery for DefaultInit<T> {
    type Data = ();
}

impl<T: Component + FromWorld> Serialize for DefaultInit<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            if !entity.contains::<T>() {
                return Err(serde::ser::Error::custom(format!(
                    "Component missing: {}.",
                    std::any::type_name::<T>()
                )));
            };
            Ok(())
        })??;
        ().serialize(serializer)
    }
}

impl<'de, T: Component + FromWorld> Deserialize<'de> for DefaultInit<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <()>::deserialize(deserializer)?;
        world_entity_scope_mut::<_, D>(|world, entity| {
            let item = T::from_world(world);
            let Ok(mut entity) = world.get_entity_mut(entity) else {
                return Err(serde::de::Error::custom("Entity missing."));
            };
            entity.insert(item);
            Ok(())
        })??;
        Ok(Self(PhantomData))
    }
}

/// Add an additional dummy [`QueryFilter`] to the [`BevyObject`] derive macro.
///
/// Use `#[serde(skip)]` to skip serializing this component completely.
pub struct AdditionalFilter<T>(PhantomData<T>);

impl<T> Debug for AdditionalFilter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultInit").finish()
    }
}

impl<T> ZstInit for AdditionalFilter<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

/// Here to make `#[serde(skip)]` work.
impl<T: QueryFilter + FromWorld> Default for AdditionalFilter<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: QueryFilter + FromWorld> BindProject for AdditionalFilter<T> {
    type To = Self;
    type Filter = T;
}

impl<T: QueryFilter + FromWorld> BindProjectQuery for AdditionalFilter<T> {
    type Data = ();
}

impl<T: QueryFilter + FromWorld> Serialize for AdditionalFilter<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ().serialize(serializer)
    }
}

impl<'de, T: QueryFilter + FromWorld> Deserialize<'de> for AdditionalFilter<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <()>::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

/// Make a [`BevyObject`] [`Deserialize`] by providing a root level entity in the world.
pub struct Root<T>(PhantomData<T>);

impl<T> Debug for Root<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Root").finish()
    }
}

impl<T> ZstInit for Root<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Root<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(Root(PhantomData))
    }
}

fn safe_despawn(world: &mut World, entity: Entity) {
    if let Ok(entity) = world.get_entity_mut(entity) {
        entity.despawn_recursive();
    }
}

impl<'de, T: BevyObject> Visitor<'de> for Root<T> {
    type Value = Root<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        loop {
            let entity = with_world_mut(|world| {
                let entity = world.spawn_empty().id();
                if let Some(mut root) = T::get_root(world) {
                    root.add_child(entity);
                }
                entity
            })
            .map_err(serde::de::Error::custom)?;
            match entity_scope(entity, || seq.next_element::<T::Object>()) {
                Err(err) => {
                    with_world_mut(|world| safe_despawn(world, entity))
                        .map_err(serde::de::Error::custom)?;
                    return Err(err);
                }
                Ok(None) => {
                    with_world_mut(|world| safe_despawn(world, entity))
                        .map_err(serde::de::Error::custom)?;
                    break;
                }
                Ok(Some(_)) => {}
            }
        }
        Ok(Root(PhantomData))
    }
}

/// Serialize a component on the active entity.
pub struct SerializeComponent<T>(PhantomData<T>);

impl<T> Debug for SerializeComponent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerializeComponent").finish()
    }
}

impl<T> ZstInit for SerializeComponent<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T: Component> BindProject for SerializeComponent<T> {
    type To = Self;
    type Filter = With<T>;
}

impl<T: Component> BindProjectQuery for SerializeComponent<T> {
    type Data = &'static T;
}

impl<T: Component + Serialize> Serialize for SerializeComponent<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            let Some(component) = entity.get::<T>() else {
                return Err(serde::ser::Error::custom(format!(
                    "Component missing: {}.",
                    std::any::type_name::<T>()
                )));
            };
            component.serialize(serializer)
        })?
    }
}

impl<'de, T: Component + Deserialize<'de>> Deserialize<'de> for SerializeComponent<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let component = T::deserialize(deserializer)?;
        world_entity_scope_mut::<_, D>(|world, entity| {
            let Ok(mut entity) = world.get_entity_mut(entity) else {
                return Err(serde::de::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            entity.insert(component);
            Ok(Self(PhantomData))
        })?
    }
}

/// Serialize a resource on the active world.
pub struct SerializeResource<T>(PhantomData<T>);

impl<T> Debug for SerializeResource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerializeResource").finish()
    }
}

impl<T> ZstInit for SerializeResource<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource + Serialize> Serialize for SerializeResource<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        with_world(|world| {
            let Some(resource) = world.get_resource::<T>() else {
                return Err(serde::ser::Error::custom(format!(
                    "Resource missing {}.",
                    std::any::type_name::<T>()
                )));
            };
            resource.serialize(serializer)
        })
        .map_err(serde::ser::Error::custom)?
    }
}

impl<'de, T: Resource + Deserialize<'de>> Deserialize<'de> for SerializeResource<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let resource = T::deserialize(deserializer)?;
        with_world_mut(|world| world.insert_resource(resource))
            .map_err(serde::de::Error::custom)?;
        Ok(Self(PhantomData))
    }
}

/// Serialize a non-send resource on the active world.
pub struct SerializeNonSend<T>(PhantomData<T>);

impl<T> ZstInit for SerializeNonSend<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T> Debug for SerializeNonSend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerializeNonSend").finish()
    }
}

impl<T: Serialize + 'static> Serialize for SerializeNonSend<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        with_world(|world| {
            let Some(resource) = world.get_non_send_resource::<T>() else {
                return Err(serde::ser::Error::custom(format!(
                    "Non-send resource missing {}.",
                    std::any::type_name::<T>()
                )));
            };
            resource.serialize(serializer)
        })
        .map_err(serde::ser::Error::custom)?
    }
}

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de> for SerializeNonSend<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let resource = T::deserialize(deserializer)?;
        with_world_mut(|world| world.insert_non_send_resource(resource))
            .map_err(serde::de::Error::custom)?;
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

impl<T> Debug for Child<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Child").finish()
    }
}

impl<T: BevyObject> BindProject for Child<T> {
    type To = Self;
    type Filter = ();
}

impl<T: BevyObject> Serialize for Child<T> {
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
            let Some(children) = entity.get::<Children>() else {
                return Err(serde::ser::Error::custom(format!(
                    "No children found for {}.",
                    type_name::<T>()
                )));
            };
            for entity in children {
                let Ok(entity) = world.get_entity(*entity) else {
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

impl<'de, T: BevyObject> Deserialize<'de> for Child<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let new_child = world_entity_scope_mut::<_, D>(|world, entity| {
            let child = world.spawn_empty().id();
            world.entity_mut(entity).add_child(child);
            child
        })?;
        entity_scope(new_child, || <T::Object>::deserialize(deserializer))
            .map_err(serde::de::Error::custom)?;
        Ok(Child(PhantomData))
    }
}

/// Extractor for multiple [`BevyObject`]s in [`Children`]
/// instead of the entity itself. This serializes children like a `Vec`.
pub struct ChildVec<T>(PhantomData<T>);

impl<T> Debug for ChildVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildVec").finish()
    }
}

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
            let children = match entity.get::<Children>() {
                Some(children) => children.as_ref(),
                None => &[],
            };
            let count = children
                .iter()
                .filter_map(|e| world.get_entity(*e).ok())
                .filter(T::filter)
                .count();
            let mut seq = serializer.serialize_seq(Some(count))?;
            for entity in children
                .iter()
                .filter_map(|e| world.get_entity(*e).ok())
                .filter(T::filter)
            {
                entity_scope(entity.id(), || seq.serialize_element(&T::init()))?;
            }
            seq.end()
        })?
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for ChildVec<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChildVec(PhantomData))
    }
}

impl<'de, T: BevyObject> Visitor<'de> for ChildVec<T> {
    type Value = ChildVec<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<Child<T>>()?.is_some() {}
        Ok(ChildVec(PhantomData))
    }
}

impl<T: BevyObject> BindProject for ChildVec<T> {
    type To = Self;
    type Filter = ();
}

/// A trait that enables [`AdaptedComponent`] to change the behavior of serialization
/// and add serialization support to non-serialize types.
pub trait SerdeAdaptor {
    type Type;

    fn serialize<S: Serializer>(serializer: S, item: &Self::Type) -> Result<S::Ok, S::Error>;
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Self::Type, D::Error>;
}

/// Apply a [`SerdeAdaptor`] to a [`SerializeComponent<T>`] to change how a component is serialized.
///
/// # Note
///
/// Non [`Serialize`] components are not [`BevyObject`], use [`SerializeComponent`] instead.
pub struct AdaptedComponent<S: SerdeAdaptor>(PhantomData<S::Type>);

impl<A: SerdeAdaptor<Type: Component>> AdaptedComponent<A> {
    #[doc(hidden)]
    pub fn serialize<S: Serializer>(
        _: &SerializeComponent<A::Type>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            let Some(component) = entity.get::<A::Type>() else {
                return Err(serde::ser::Error::custom(format!(
                    "Component missing: {}.",
                    std::any::type_name::<A::Type>()
                )));
            };
            A::serialize(serializer, component)
        })?
    }

    #[doc(hidden)]
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<SerializeComponent<A::Type>, D::Error> {
        let component = A::deserialize(deserializer)?;
        world_entity_scope_mut::<_, D>(|world, entity| {
            let Ok(mut entity) = world.get_entity_mut(entity) else {
                return Err(serde::de::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            entity.insert(component);
            Ok(SerializeComponent(PhantomData))
        })?
    }
}
