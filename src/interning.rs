//! Module for interning data in a [`Resource`].
use std::any::type_name;
use std::ops::Deref;

use bevy_ecs::system::Resource;
use ref_cast::RefCast;
use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;

use crate::with_world;

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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, RefCast)]
#[repr(transparent)]
pub struct Interned<T: InterningKey>(pub T);

impl<T: InterningKey> Deref for Interned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: InterningKey> Serialize for Interned<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        with_world::<_, S>(|world| {
            match world.get_resource::<T::Interner>() {
                Some(interner) => match interner.get(&self.0) {
                    Ok(value) => value.serialize(serializer),
                    Err(err) => Err(serde::ser::Error::custom(err)),
                },
                None => Err(serde::ser::Error::custom(
                    format!("Interner resource {} missing.", type_name::<T::Interner>())
                )),
            }
        })?
    }
}
