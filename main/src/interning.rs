use std::borrow::Borrow;
use bevy_ecs::system::Resource;
use ref_cast::RefCast;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::{BoxError, Convert, FromWorldAccess, SerdeProject};

/// A key to a value in an [`Interner`] [`Resource`].
pub trait InterningKey: 'static {
    /// The type of value this key represents.
    type Represents;
    type Interner: Interner<Key=Self, Value=Self::Represents>;
}

/// A [`Resource`] that holds a pool of values accessible by a [`InterningKey`].
pub trait Interner: Resource {
    type Key;
    type Value;

    /// Obtain an existing value.
    fn get(&self, key: &Self::Key) -> Result<&Self::Value, BoxError>;
    fn add(&self, value: Self::Value) -> Result<Self::Key, BoxError>;
}

/// Serialize an [`InterningKey`] based on the interned value.
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

impl<T: InterningKey> SerdeProject for Interned<T> where T::Represents: Serialize + DeserializeOwned {
    type Ctx = T::Interner;
    type Ser<'t> = &'t T::Represents;
    type De<'de> = T::Represents;

    fn to_ser<'t>(&'t self, ctx: <Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        ctx.get(&self.0)
    }

    fn from_de(ctx: <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(ctx.add(de)?))
    }
}