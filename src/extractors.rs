use crate::{BevyObject, BindProject, BindProjectQuery, MappedSerializer, ZstInit};
use bevy::ecs::{
    query::{QueryFilter, With},
    resource::Resource,
    world::FromWorld,
};
use bevy::state::state::{FreelyMutableState, NextState, State};
use bevy_serde_lens_core::{DeUtils, SerUtils};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Debug, marker::PhantomData};

#[allow(unused)]
use bevy::ecs::component::Component;

/// A unified marker wrapper for a few things.
///
/// When use in the [`BevyObject`] macro, allows mandatory items to be missing
/// and serialize them as an [`Option`].
/// `#[serde(default)]` can be used to make this optional
/// if used in self describing formats.
///
/// When used in `with` notation inside a serde macro,
/// allows certain modifiers to take `Option<T>` instead of `T`.
pub struct Maybe<T>(pub(crate) PhantomData<T>);

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
        SerUtils::with_entity_ref::<S, _>(|entity| {
            if T::filter(&entity) {
                Some(T::init())
            } else {
                None
            }
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

/// Doesn't do anything since we discard errors.
type DummyDeserializer = serde::de::value::BoolDeserializer<serde::de::value::Error>;

/// To make `#[serde(default)]` work.
impl<T: Component + FromWorld> Default for DefaultInit<T> {
    fn default() -> Self {
        let Ok(entity) = DeUtils::current_entity::<DummyDeserializer>() else {
            return Self(PhantomData);
        };
        let _ = DeUtils::with_world_mut::<DummyDeserializer, _>(|world| {
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
        // We need to validate the component exists.
        SerUtils::with_component::<T, S, _>(|_| ())?;
        ().serialize(serializer)
    }
}

impl<'de, T: Component + FromWorld> Deserialize<'de> for DefaultInit<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <() as Deserialize>::deserialize(deserializer)?;
        let item = DeUtils::with_world_mut::<D, _>(T::from_world)?;
        DeUtils::insert::<D>(item)?;
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
        <() as Deserialize>::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

/// Serialize a component on the active entity.
pub struct AdaptedComponent<T, C>(PhantomData<(T, C)>);

/// Serialize a component on the active entity.
pub type SerializeComponent<T> = AdaptedComponent<T, ()>;

impl<T, C> Debug for AdaptedComponent<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerializeComponent").finish()
    }
}

impl<T, C> ZstInit for AdaptedComponent<T, C> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T: Component, C> BindProject for AdaptedComponent<T, C> {
    type To = Self;
    type Filter = With<T>;
}

impl<T: Component, C> BindProjectQuery for AdaptedComponent<T, C> {
    type Data = &'static T;
}

impl<T: Component, C: MappedSerializer<T>> Serialize for AdaptedComponent<T, C> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerUtils::with_component::<T, S, _>(|component| C::serialize(component, serializer))?
    }
}

impl<'de, T: Component, C: MappedSerializer<T>> Deserialize<'de> for AdaptedComponent<T, C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let component = C::deserialize(deserializer)?;
        DeUtils::insert::<D>(component)?;
        Ok(ZstInit::init())
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
        SerUtils::with_resource::<T, S, _>(|resource| resource.serialize(serializer))?
    }
}

impl<'de, T: Resource + Deserialize<'de>> Deserialize<'de> for SerializeResource<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let resource = T::deserialize(deserializer)?;
        DeUtils::with_world_mut::<D, _>(|world| {
            world.insert_resource(resource);
        })?;
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
        SerUtils::with_non_send_resource::<T, S, _>(|resource| resource.serialize(serializer))?
    }
}

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de> for SerializeNonSend<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let resource = T::deserialize(deserializer)?;
        DeUtils::with_world_mut::<D, _>(|world| {
            world.insert_non_send_resource(resource);
        })?;
        Ok(Self(PhantomData))
    }
}

/// Serialize a resource on the active world.
pub struct SerializeState<T>(PhantomData<T>);

impl<T> Debug for SerializeState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerializeState").finish()
    }
}

impl<T> ZstInit for SerializeState<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<T: FreelyMutableState + Serialize> Serialize for SerializeState<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerUtils::with_resource::<State<T>, S, _>(|resource| resource.get().serialize(serializer))?
    }
}

impl<'de, T: FreelyMutableState + Deserialize<'de>> Deserialize<'de> for SerializeState<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let new_state = T::deserialize(deserializer)?;
        DeUtils::with_resource_mut::<NextState<T>, D, _>(|mut state| {
            state.set(new_state);
        })?;
        Ok(Self(PhantomData))
    }
}
