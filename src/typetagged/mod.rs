//! The `typetag` crate allows you to serialize trait objects like `Box<dyn T>`,
//! but using static linking or life before main has its own issues
//! and these methods currently does not work on wasm.
//!
//! To address these limitations this crate allows you to register deserializers manually
//! in the bevy `World` and use the `TypeTagged` newtype for serialization.
//!
//! ```rust
//! impl ErasedObject for Box<dyn Animal> {
//!     ..
//! }
//!
//! world.register_typetag::<Box<dyn Animal>, Cat>()
//! ```
//!
//! then
//!
//! ```rust
//! #[derive(Serialize, Deserialize)]
//! struct MyComponent {
//!     #[serde(with = "TypeTagged")]
//!     animal: Box<dyn Animal>
//! }
//! ```
//!
//! [`TypeTagged`] and similar types can be used in the `BevyObject` derive macro as well:
//!
//! ```rust
//! #[derive(BevyObject)]
//! struct SerializeAnimal {
//!     name: Name,
//!     // serialize component `Box<dyn Animal>` using `TypeTagged`.
//!     animal: TypeTagged<Box<dyn Animal>>,
//! }
//! ```
//!
//! To have more user friendly configuration files, we can leverage serde's `deserialize_any` by
//! implement additional methods on [`ErasedObject`] and
//! use [`AnyOrTagged`] or [`SmartTagged`].
mod internal;
pub(crate) use internal::TYPETAG_SERVER;
pub use internal::TypeTagServer;
use internal::TypeTaggedVisitor;
mod bevy_object;
use ref_cast::RefCast;
use serde::{Deserialize, Serialize};
use std::{
    any::type_name,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::impl_with_notation_newtype;

/// A serializable trait object of an [`ErasedObject`].
///
/// Serialization is done in [`ErasedObject`]
/// and deserialization is done via registered deserializers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, RefCast)]
#[repr(transparent)]
pub struct TypeTagged<T>(pub T);

impl<T> Deref for TypeTagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TypeTagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A serializable trait object that can use `deserialize_any`.
///
/// # Why
///
/// Normally [`TypeTagged`] deserializes from something like
///
/// ```
/// {
///     "my_field": {
///         "f32": 1.23
///     }
/// }
/// ```
///
/// This might be cumbersome for human written data so we allow parsing non-maps directly
/// using `deserialize_any`
///
/// ```
/// {
///     "my_field": 1.23
/// }
/// ```
///
/// # Add Serializers
///
/// Add serializers to [`ErasedObject::try_serialize_any`], add deserializers to
/// by implementing `deserialize_*` functions on [`ErasedObject`].
///
/// # Note
///
/// Due to the serde specification this is not allowed on non-self-describing formats
/// like `postcard` and will cause an error, be careful when using this in multiple formats.
/// See [`SmartTagged`] as a non-serde standard alternative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, RefCast)]
#[repr(transparent)]
pub struct AnyOrTagged<T>(pub T);

impl<T> Deref for AnyOrTagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AnyOrTagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A serializable trait object that uses `deserialize_any` when serializer is human readable.
///
/// See [`AnyOrTagged`] for explanation.
///
/// This is not based on the serde specification so it might fail on niche formats
/// that are human readable but not self describing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, RefCast)]
#[repr(transparent)]
pub struct SmartTagged<T>(pub T);

impl<T> Deref for SmartTagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SmartTagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Error in deserialization.
#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    #[error("{function} unimplemented for {ty}.")]
    Unimplemented {
        ty: &'static str,
        function: &'static str,
    },
    #[error("{0}")]
    Custom(String),
}

/// A type erased object like `Box<dyn T>` that is (de)serializable with world access.
///
/// # Note:
///
/// Implementing this trait only makes serialization work,
/// not deserialization. You need to call `register_typetag`
/// on `World` or `App` with concrete subtypes for deserialization.
///
/// # Example
///
/// A simple setup to serialize and deserialize a dynamic stat `Box<dyn Stat>`.
/// ```
/// pub trait Stat: DynamicTypePath {
///     fn as_serialize(&self) -> &dyn erased_serde::Serialize;
/// }
///
/// impl BevyTypeTagged for Box<dyn Stat> {
///     fn name(&self) -> &'static str {
///         self.reflect_short_type_path()
///     }
///
///     fn as_serialize(&self) -> &dyn erased_serde::Serialize {
///         Stat::as_serialize(self)
///     }
/// }
///
/// impl<T: Stat> Into<Box<dyn Stat>> for T {
///     fn into(value: Self) -> Box<dyn Stat> {
///         Box::new(value)
///     }
/// }
///
/// #[derive(Serialize, Deserialize)]
/// pub struct MyStat { .. }
///
/// impl Stat for MyStat { .. }
///
/// fn my_main() {
///     ..
///     app.register_typetag::<Box<dyn<Stat>>, MyStat>   
/// }
/// ```
#[allow(unused_variables)]
pub trait ErasedObject: Sized + 'static {
    /// Returns the type name of the implementor.
    fn name(&self) -> impl AsRef<str>;

    /// Returns the untagged inner value of the implementor.
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;

    /// If using `deserialize_any` mode, try serialize in an untagged format,
    /// must match one of the `deserialize_*` functions implemented in this trait.
    ///
    /// Falls back to serializing an externally tagged map if fails.
    fn try_serialize_any(&self) -> Option<&dyn erased_serde::Serialize> {
        None
    }

    fn deserialize_unit() -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_unit",
        })
    }

    fn deserialize_bool(value: bool) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_bool",
        })
    }

    fn deserialize_uint(value: u64) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_uint",
        })
    }

    fn deserialize_int(value: i64) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_int",
        })
    }

    fn deserialize_float(value: f64) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_float",
        })
    }

    fn deserialize_char(value: char) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_char",
        })
    }

    fn deserialize_string(value: &str) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_string",
        })
    }

    fn deserialize_bytes(value: &[u8]) -> Result<Self, DeserializeError> {
        Err(DeserializeError::Unimplemented {
            ty: type_name::<Self>(),
            function: "deserialize_bytes",
        })
    }
}

impl<V> serde::Serialize for TypeTagged<V>
where
    V: ErasedObject,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.0.name().as_ref(), &self.0.as_serialize())?;
        map.end()
    }
}

impl<V> serde::Serialize for AnyOrTagged<V>
where
    V: ErasedObject,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        if let Some(ser) = V::try_serialize_any(self) {
            ser.serialize(serializer)
        } else {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry(self.0.name().as_ref(), &self.0.as_serialize())?;
            map.end()
        }
    }
}

impl<V> serde::Serialize for SmartTagged<V>
where
    V: ErasedObject,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        if serializer.is_human_readable() {
            if let Some(ser) = V::try_serialize_any(self) {
                return ser.serialize(serializer);
            }
        }
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.0.name().as_ref(), &self.0.as_serialize())?;
        map.end()
    }
}

impl<'de, V: ErasedObject> serde::Deserialize<'de> for TypeTagged<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_map(TypeTaggedVisitor::<V>(PhantomData))
            .map(TypeTagged)
    }
}

impl<'de, V: ErasedObject> serde::Deserialize<'de> for AnyOrTagged<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_any(TypeTaggedVisitor::<V>(PhantomData))
            .map(AnyOrTagged)
    }
}

impl<'de, V: ErasedObject> serde::Deserialize<'de> for SmartTagged<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_any(TypeTaggedVisitor::<V>(PhantomData))
                .map(SmartTagged)
        } else {
            deserializer
                .deserialize_map(TypeTaggedVisitor::<V>(PhantomData))
                .map(SmartTagged)
        }
    }
}

impl_with_notation_newtype!([T: ErasedObject] TypeTagged [T] T);
impl_with_notation_newtype!([T: ErasedObject] AnyOrTagged [T] T);
impl_with_notation_newtype!([T: ErasedObject] SmartTagged [T] T);
