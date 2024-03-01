//! Module for interning data in a [`Resource`].

use std::borrow::Borrow;
use bevy_ecs::system::Resource;
use ref_cast::RefCast;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::{BoxError, Convert, FromWorldAccess, SerdeProject};

/// A key to a value in an [`Interner`] resource.
pub trait InterningKey: Sized + 'static {
    /// The type of value this key represents.
    type Value;
    type Interner: Interner<Self, Value=Self::Value>;
}

/// A [`Resource`] that holds a pool of values accessible by a [`InterningKey`].
pub trait Interner<Key>: Resource {
    type Value;

    /// Obtain an existing value.
    fn get(&self, key: &Key) -> Result<&Self::Value, BoxError>;
    fn add(&mut self, value: Self::Value) -> Result<Key, BoxError>;
}

/// Projection of an [`InterningKey`] that serializes the interned value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, RefCast)]
#[repr(transparent)]
pub struct Interned<T: InterningKey>(T);

impl<T: InterningKey> Convert<T> for Interned<T> {
    fn ser(input: &T) -> impl Borrow<Self> {
        Self::ref_cast(input)
    }

    fn de(self) -> T {
        self.0
    }
}

impl<T: InterningKey> SerdeProject for Interned<T> where T::Value: Serialize + DeserializeOwned {
    type Ctx = T::Interner;
    type Ser<'t> = &'t T::Value;
    type De<'de> = T::Value;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        ctx.get(&self.0)
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(ctx.add(de)?))
    }
}
