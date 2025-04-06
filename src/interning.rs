//! Module for interning data in a [`Resource`].
use std::ops::Deref;
use std::ops::DerefMut;

use bevy_ecs::resource::Resource;
use bevy_serde_lens_core::DeUtils;
use bevy_serde_lens_core::SerUtils;
use ref_cast::RefCast;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

/// A key to a value in an [`Interner`] resource.
pub trait InterningKey: Sized + 'static {
    type Interner: Interner<Self>;
}

/// A [`Resource`] that holds a pool of values accessible by a [`InterningKey`].
pub trait Interner<Key>: Resource {
    type Error: std::error::Error;
    type ValueRef<'t>: Serialize;
    type Value<'de>: Deserialize<'de>;

    /// Obtain an existing value.
    fn get(&self, key: &Key) -> Result<Self::ValueRef<'_>, Self::Error>;
    fn add(&mut self, value: Self::Value<'_>) -> Result<Key, Self::Error>;
}

/// Projection of an [`InterningKey`] that serializes the interned value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, RefCast)]
#[repr(transparent)]
pub struct Interned<T: InterningKey>(pub T);

impl<T: InterningKey> Deref for Interned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: InterningKey> DerefMut for Interned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: InterningKey> Serialize for Interned<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerUtils::with_resource::<T::Interner, S, _>(|interner| match interner.get(&self.0) {
            Ok(value) => value.serialize(serializer),
            Err(err) => Err(SerUtils::error::<S>(err)),
        })?
    }
}

impl<'de, T: InterningKey> Deserialize<'de> for Interned<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <<T::Interner as Interner<T>>::Value<'de>>::deserialize(deserializer)?;
        DeUtils::with_resource_mut::<T::Interner, D, _>(|mut interner| match interner.add(value) {
            Ok(value) => Ok(Interned(value)),
            Err(err) => Err(DeUtils::error::<D>(err)),
        })?
    }
}

impl<T: InterningKey> Interned<T> {
    pub fn serialize<S: Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error> {
        Interned::ref_cast(item).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        <Interned<T> as Deserialize>::deserialize(deserializer).map(|x| x.0)
    }
}
