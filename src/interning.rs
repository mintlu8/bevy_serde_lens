//! Module for interning data in a [`Resource`].
use bevy_ecs::system::Resource;
use ref_cast::RefCast;
use serde::Deserialize;
use serde::Serialize;
use crate::{BoxError, Convert, FromWorldAccess, SerdeProject};

/// A key to a value in an [`Interner`] resource.
pub trait InterningKey: Sized + 'static {
    type Interner: Interner<Self>;
}

/// A [`Resource`] that holds a pool of values accessible by a [`InterningKey`].
pub trait Interner<Key>: Resource {
    type ValueRef<'t>;
    type Value<'de>;

    /// Obtain an existing value.
    fn get(&self, key: &Key) -> Result<Self::ValueRef<'_>, BoxError>;
    fn add(&mut self, value: Self::Value<'_>) -> Result<Key, BoxError>;
}

/// Projection of an [`InterningKey`] that serializes the interned value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, RefCast)]
#[repr(transparent)]
pub struct Interned<T: InterningKey>(pub T);

impl<T: InterningKey> Convert<T> for Interned<T> {
    fn ser(input: &T) -> &Self {
        Self::ref_cast(input)
    }

    fn de(self) -> T {
        self.0
    }
}

impl<T: InterningKey> SerdeProject for Interned<T> where
        for<'t> <T::Interner as Interner<T>>::ValueRef<'t>: Serialize,
        for<'de> <T::Interner as Interner<T>>::Value<'de>: Deserialize<'de> {
    type Ctx = T::Interner;
    type Ser<'t> = <T::Interner as Interner<T>>::ValueRef<'t>;
    type De<'de> = <T::Interner as Interner<T>>::Value<'de>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        ctx.get(&self.0)
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(ctx.add(de)?))
    }
}
