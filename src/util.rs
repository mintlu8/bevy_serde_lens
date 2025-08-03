use std::marker::PhantomData;

use ref_cast::RefCast;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

/// Add or modify serialization of an external type.
pub trait MappedSerializer<T> {
    fn serialize<S: Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error>;
    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error>;
}

impl<T: Serialize + DeserializeOwned> MappedSerializer<T> for () {
    fn serialize<S: Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error> {
        item.serialize(serializer)
    }

    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        T::deserialize(deserializer)
    }
}

#[derive(RefCast)]
#[repr(transparent)]
pub(crate) struct MappedValue<T, M: MappedSerializer<T>>(pub T, PhantomData<M>);

impl<T, M: MappedSerializer<T>> Serialize for MappedValue<T, M> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        M::serialize(&self.0, serializer)
    }
}

impl<'de, T, M: MappedSerializer<T>> Deserialize<'de> for MappedValue<T, M> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(MappedValue(M::deserialize(deserializer)?, PhantomData))
    }
}

macro_rules! impl_with_notation_newtype {
    ([$($impl_g:tt)*] $name: ident [$($ty_g: tt)*] $inner: ty) => {
        #[doc(hidden)]
        impl<$($impl_g)*> $name<$($ty_g)*> {
            pub fn serialize<S: ::serde::Serializer>(this: &$inner, serializer: S) -> Result<S::Ok, S::Error> {
                use ::ref_cast::RefCast;
                Serialize::serialize(Self::ref_cast(this), serializer)
            }

            pub fn deserialize<'de, D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<$inner, D::Error> {
                <Self as Deserialize>::deserialize(deserializer).map(|x| x.0)
            }
        }

        #[doc(hidden)]
        impl<$($impl_g)*> $crate::Maybe<$name<$($ty_g)*>> {
            pub fn serialize<S: ::serde::Serializer>(this: &Option<$inner>, serializer: S) -> Result<S::Ok, S::Error> {
                use ::ref_cast::RefCast;
                Serialize::serialize(&this.as_ref().map($name::<$($ty_g)*>::ref_cast), serializer)
            }

            pub fn deserialize<'de, D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Option<$inner>, D::Error> {
                <Option<$name::<$($ty_g)*>> as Deserialize>::deserialize(deserializer).map(|x| x.map(|x| x.0))
            }
        }
    };
}

pub(crate) use impl_with_notation_newtype;

/// Format a [`serde::ser::Error`].
#[macro_export]
macro_rules! serrorf {
    ($($tt: tt)*) => {
        $crate::serde::ser::Error::custom($crate::format!($($tt)*))
    };
}

/// Format a [`serde::de::Error`].
#[macro_export]
macro_rules! derrorf {
    ($($tt: tt)*) => {
        $crate::serde::de::Error::custom($crate::format!($($tt)*))
    };
}
