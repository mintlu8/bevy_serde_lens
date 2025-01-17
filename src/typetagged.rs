//! Module for deserializing artificial objects.
//!
//! # Getting Started
//!
//! Let's serialize this struct:
//!
//! ```
//! struct StatEntry {
//!     stat: Box<dyn Stat>,
//!     value: u32,
//! }
//! ```
//!
//! ```
//! // Implement `TypeTagged` for Box<dyn Stat>
//! impl BevyTypeTagged for Box<dyn Stat> {
//!     ...
//! }
//!
//! // StatA is an implementation of Stat
//! impl Stat for StatA {
//!     ...
//! }
//!
//! // Implement `IntoTypeTagged` for specific implementations of `Stat`
//! impl IntoTypeTagged for StatA {
//!     ...
//! }
//!
//! // Register specific implementations on the `World`
//! fn my_main() {
//!     ..
//!     app.register_typetag::<Box<dyn<Stat>>, StatA>   
//!     app.register_typetag::<Box<dyn<Stat>>, StatB>   
//! }
//! ```
//!
//! Then derive [`Serialize`] and [`Deserialize`] on `StatEntry`:
//!
//! ```
//! #[derive(Serialize, Deserialize)]
//! struct StatEntry {
//!     #[serde(with = "TypeTagged")]
//!     stat: Box<dyn Stat>,
//!     value: u32,
//! }
//! ```
//!
//! # Deserialize Any
//!
//! Use [`register_deserialize_any`](crate::WorldExtension::register_deserialize_any) to add functions
//! to deserialize from primitives like `i64`, `str`, etc.
//!
//! Normally the format is, in json:
//!
//! ```
//! {
//!     "field": {
//!         "Type": "Value"
//!     }
//! }
//! ```
//!
//! Using deserialize any, this can be simplified as
//!
//! ```
//! {
//!     "field": "Value"
//! }
//! ```
//!
//! Keep in mind calling `deserialize_any` will always
//! panic in non-self describing formats like `postcard`,
//! as this is a limitation of the serde specification. Therefore [`TypeTagged`]
//! will never call `deserialize_any`. Use [`AnyTagged`] to use `deserialize_any`
//! on primitives.

use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use erased_serde::Deserializer;
use ref_cast::RefCast;
use rustc_hash::FxHashMap;
use serde::{
    de::{DeserializeOwned, DeserializeSeed, Visitor},
    Deserialize, Serialize,
};
use std::{
    any::{type_name, Any, TypeId},
    borrow::Cow,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

scoped_tls_hkt::scoped_thread_local! {
    pub(crate) static TYPETAG_SERVER: TypeTagServer
}

/// A serializable trait object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, RefCast)]
#[repr(transparent)]
pub struct TypeTagged<T: ErasedObject>(pub T);

impl<T: ErasedObject> Deref for TypeTagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ErasedObject> DerefMut for TypeTagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A serializable trait object that uses `deserialize_any`.
///
/// Serialize has the same behavior as [`TypeTagged`].
///
/// # Why
///
/// Normally [`TypeTagged`] deserializes from something like
///
/// ```
/// {
///     "my_field": {
///         "TypeName": 1.23
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
/// Due to the serde specification this is not allowed on non-self-describing formats
/// like `postcard` and will cause an error, be careful when using this in multiple formats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, RefCast)]
#[repr(transparent)]
pub struct AnyTagged<T: ErasedObject>(pub T);

impl<T: ErasedObject> Deref for AnyTagged<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ErasedObject> DerefMut for AnyTagged<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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

/// A trait object like `Box<dyn T>` that is (de)serializable with world access.
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
/// pub trait Stat {
///     fn name(&self) -> &'static str;
///     fn as_serialize(&self) -> &dyn erased_serde::Serialize;
/// }
///
/// impl BevyTypeTagged for dyn Stat {
///     fn name(&self) -> &'static str {
///         Stat::name(self)
///     }
///
///     fn as_serialize(&self) -> &dyn erased_serde::Serialize {
///         Stat::as_serialize(self)
///     }
/// }
///
/// #[derive(Serialize, Deserialize)]
/// pub struct MyStat { .. }
///
/// impl Stat for MyStat { .. }
///
/// impl IntoTypeTagged<Box<dyn Stat>> for MyStat { .. }
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
    /// must match one of the `deserialize_*` functions in this trait.
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

type DeserializeFn<T> = fn(&mut dyn erased_serde::Deserializer) -> Result<T, erased_serde::Error>;

/// A [`Resource`] that stores registered deserialization functions.
#[derive(Resource, Default)]
pub struct TypeTagServer {
    functions: FxHashMap<(TypeId, Cow<'static, str>), Box<dyn Any + Send + Sync>>,
}

impl TypeTagServer {
    pub fn get<T: ErasedObject>(&self, name: &str) -> Option<DeserializeFn<T>> {
        let id = TypeId::of::<T>();
        self.functions
            .get(&(id, Cow::Borrowed(name)))
            .and_then(|f| f.downcast_ref())
            .copied()
    }

    pub fn clear(&mut self) {
        self.functions.clear();
    }

    pub fn register<T: ErasedObject, A: Into<T> + DeserializeOwned + TypePath>(&mut self) {
        let id = TypeId::of::<T>();
        let de_fn: DeserializeFn<T> = |de| Ok(erased_serde::deserialize::<A>(de)?.into());
        self.functions.insert(
            (id, Cow::Owned(A::short_type_path().to_owned())),
            Box::new(de_fn),
        );
    }

    pub fn register_by_name<T: ErasedObject, A: Into<T> + DeserializeOwned>(&mut self, name: &str) {
        let id = TypeId::of::<T>();
        let de_fn: DeserializeFn<T> = |de| Ok(erased_serde::deserialize::<A>(de)?.into());
        self.functions
            .insert((id, Cow::Owned(name.to_owned())), Box::new(de_fn));
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

impl<V> serde::Serialize for AnyTagged<V>
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

impl<'de, V: ErasedObject> serde::Deserialize<'de> for AnyTagged<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_any(TypeTaggedVisitor::<V>(PhantomData))
            .map(AnyTagged)
    }
}

struct TypeTaggedVisitor<'de, V: ErasedObject>(PhantomData<&'de V>);

impl<'de, V: ErasedObject> Visitor<'de> for TypeTaggedVisitor<'de, V> {
    type Value = V;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "externally tagged enum")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let Some(key) = map.next_key::<Cow<str>>()? else {
            return Err(serde::de::Error::custom("expected externally tagged value"));
        };
        if !TYPETAG_SERVER.is_set() {
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        let Some(de_fn) = TYPETAG_SERVER.with(|map| map.get::<V>(&key)) else {
            return Err(serde::de::Error::custom(format!(
                "unregistered type-tag {}",
                key,
            )));
        };
        map.next_value_seed(DeserializeFnSeed(de_fn, PhantomData))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_unit().map_err(serde::de::Error::custom)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_bool(v).map_err(serde::de::Error::custom)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_int(v).map_err(serde::de::Error::custom)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_uint(v).map_err(serde::de::Error::custom)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_float(v).map_err(serde::de::Error::custom)
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_char(v).map_err(serde::de::Error::custom)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_string(v).map_err(serde::de::Error::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        V::deserialize_bytes(v).map_err(serde::de::Error::custom)
    }
}

struct DeserializeFnSeed<'de, T: ErasedObject>(DeserializeFn<T>, PhantomData<&'de ()>);

impl<'de, T: ErasedObject> DeserializeSeed<'de> for DeserializeFnSeed<'de, T> {
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        (self.0)(&mut <dyn Deserializer>::erase(deserializer)).map_err(serde::de::Error::custom)
    }
}

impl<T: ErasedObject> TypeTagged<T> {
    /// Serialize with [`TypeTagged`].
    pub fn serialize<S: serde::Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error> {
        TypeTagged::ref_cast(item).serialize(serializer)
    }

    /// Deserialize with [`TypeTagged`].
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        <TypeTagged<T> as Deserialize>::deserialize(deserializer).map(|x| x.0)
    }
}

impl<T: ErasedObject> AnyTagged<T> {
    /// Serialize with [`AnyTagged`].
    pub fn serialize<S: serde::Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error> {
        AnyTagged::ref_cast(item).serialize(serializer)
    }

    /// Deserialize with [`AnyTagged`].
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        <AnyTagged<T> as Deserialize>::deserialize(deserializer).map(|x| x.0)
    }
}
