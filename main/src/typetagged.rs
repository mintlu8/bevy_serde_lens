//! Module for serializing trait objects.
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
//! Then derive [`SerdeProject`](::bevy_serde_project_derive::SerdeProject) on `StatEntry`:
//!
//! ```
//! #[derive(SerdeProject)]
//! struct StatEntry {
//!     #[serde_project("TypeTagged<Box<dyn Stat>>")]
//!     stat: Box<dyn Stat>,
//!     value: u32,
//! }
//! ```


use std::{any::{Any, TypeId}, borrow::Cow, marker::PhantomData};
use bevy_ecs::system::Resource;
use erased_serde::Deserializer;
use ref_cast::RefCast;
use rustc_hash::FxHashMap;
use scoped_tls::scoped_thread_local;
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use crate::Convert;

scoped_thread_local! {
    static DESERIALIZE_FUNCTIONS: TypeTagServer
}

scoped_thread_local!(
    static DESERIALIZE_ANY_FUNCTIONS: DeserializeAnyServer
);

pub(crate) fn scoped<T>(deserialize_fns: &TypeTagServer, f: impl FnOnce() -> T) -> T{
    DESERIALIZE_FUNCTIONS.set(deserialize_fns, f)
}

pub(crate) fn scoped_any<T>(deserialize_fns: &DeserializeAnyServer, f: impl FnOnce() -> T) -> T{
    DESERIALIZE_ANY_FUNCTIONS.set(deserialize_fns, f)
}

/// A serializable trait object.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct TypeTagged<T: BevyTypeTagged>(T);

impl<T: BevyTypeTagged> Convert<T> for TypeTagged<T> {
    fn ser(input: &T) -> &Self {
        TypeTagged::ref_cast(input)
    }

    fn de(self) -> T {
        self.0
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
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct AnyTagged<T: BevyTypeTagged>(T);

impl<T: BevyTypeTagged> Convert<T> for AnyTagged<T> {
    fn ser(input: &T) -> &Self {
        AnyTagged::ref_cast(input)
    }

    fn de(self) -> T {
        self.0
    }
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
/// impl BevyTypeTagged for Box<dyn Stat> {
///     fn name(&self) -> &'static str {
///         self.as_ref().name()
///     }
///
///     fn as_serialize(&self) -> &dyn erased_serde::Serialize {
///         self.as_ref().as_serialize()
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
pub trait BevyTypeTagged: Send + Sync + 'static {
    /// Returns the type name of the implementor.
    fn name(&self) -> impl AsRef<str>;
    /// Returns the untagged inner value of the implementor.
    ///
    /// # Note
    ///
    /// If you used the actual `typetag` crate on your trait, be sure to use
    /// return a reference to the inner value instead of `dyn YourTrait`.
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

/// A concrete type that implements a [`BevyTypeTagged`] trait.
pub trait FromTypeTagged<T: DeserializeOwned>: BevyTypeTagged {
    /// Type name, must be unique per type and 
    /// must match the output on the corresponding [`BevyTypeTagged`]
    /// when type erased.
    fn name() -> impl AsRef<str>;
    /// Convert to a [`BevyTypeTagged`] type.
    fn from_type_tagged(item: T) -> Self;
}

/// A concrete type that implements a [`BevyTypeTagged`] trait.
pub trait IntoTypeTagged<T: BevyTypeTagged>: DeserializeOwned {
    /// Type name, must be unique per type and 
    /// must match the output on the corresponding [`BevyTypeTagged`]
    /// when type erased.
    fn name() -> impl AsRef<str>;
    /// Convert to a [`BevyTypeTagged`] type.
    fn into_type_tagged(self) -> T;
}

impl<T: BevyTypeTagged, U: DeserializeOwned> IntoTypeTagged<T> for U where T: FromTypeTagged<U> {
    fn name() -> impl AsRef<str> {
        <T as FromTypeTagged<U>>::name()
    }

    fn into_type_tagged(self) -> T {
        T::from_type_tagged(self)
    }
}

type DeserializeFn<T> = fn(&mut dyn erased_serde::Deserializer) -> Result<T, erased_serde::Error>;

/// A [`Resource`] that stores registered deserialization functions.
#[derive(Resource, Default)]
pub struct TypeTagServer {
    functions: FxHashMap<(TypeId, Cow<'static, str>), Box<dyn Any + Send + Sync>>,
}

impl TypeTagServer {
    pub fn get<T: BevyTypeTagged>(&self, name: &str) -> Option<DeserializeFn<T>>{
        let id = TypeId::of::<T>();
        self.functions.get(&(id, Cow::Borrowed(name))).and_then(|f| f.downcast_ref()).copied()
    }

    pub fn clear(&mut self) {
        self.functions.clear()
    }

    pub fn register<T: BevyTypeTagged, A: IntoTypeTagged<T>>(&mut self) {
        let id = TypeId::of::<T>();
        let de_fn: DeserializeFn<T> = |de| {
            Ok(A::into_type_tagged(erased_serde::deserialize::<A>(de)?))
        };
        self.functions.insert((id, Cow::Owned(A::name().as_ref().to_owned())), Box::new(de_fn));
    }
}

type DeserializeUnitFn<T> = fn() -> Result<T, String>;
type DeserializeBoolFn<T> = fn(bool) -> Result<T, String>;
type DeserializeIntFn<T> = fn(i64) -> Result<T, String>;
type DeserializeUIntFn<T> = fn(u64) -> Result<T, String>;
type DeserializeFloatFn<T> = fn(f64) -> Result<T, String>;
type DeserializeCharFn<T> = fn(char) -> Result<T, String>;
type DeserializeStrFn<T> = fn(&str) -> Result<T, String>;
type DeserializeBytesFn<T> = fn(&[u8]) -> Result<T, String>;

// Experimental
// type DeserializeSeqFn<T> = fn(&mut dyn erased_serde::Deserializer) -> Result<T, erased_serde::Error>;

/// A [`Resource`] that stores registered deserialize functions from primitives.
#[derive(Resource, Default)]
pub struct DeserializeAnyServer {
    deserialize_unit: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_bool: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_int: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_uint: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_float: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_char: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_str: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_bytes: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
    deserialize_seq: FxHashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl DeserializeAnyServer {
    pub fn clear(&mut self) {
        self.deserialize_unit.clear();
        self.deserialize_bool.clear();
        self.deserialize_int.clear();
        self.deserialize_uint.clear();
        self.deserialize_float.clear();
        self.deserialize_char.clear();
        self.deserialize_str.clear();
        self.deserialize_bytes.clear();
        self.deserialize_seq.clear();
    }

    pub fn register_unit<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeUnitFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_unit.insert(id, Box::new(f));
    }

    pub fn register_bool<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeBoolFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_bool.insert(id, Box::new(f));
    }

    pub fn register_int<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeIntFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_int.insert(id, Box::new(f));
    }

    pub fn register_uint<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeUIntFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_uint.insert(id, Box::new(f));
    }

    pub fn register_float<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeFloatFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_float.insert(id, Box::new(f));
    }

    pub fn register_char<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeCharFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_char.insert(id, Box::new(f));
    }

    pub fn register_str<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeStrFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_str.insert(id, Box::new(f));
    }

    pub fn register_bytes<T: BevyTypeTagged, A: Send + Sync + 'static>(&mut self, f: DeserializeBytesFn<A>) {
        let id = TypeId::of::<T>();
        self.deserialize_bytes.insert(id, Box::new(f));
    }
}

impl<V> serde::Serialize for TypeTagged<V> where V: BevyTypeTagged {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.0.name().as_ref(), &self.0.as_serialize())?;
        map.end()
    }
}

impl<V> serde::Serialize for AnyTagged<V> where V: BevyTypeTagged {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.0.name().as_ref(), &self.0.as_serialize())?;
        map.end()
    }
}

impl<'de, V: BevyTypeTagged> serde::Deserialize<'de> for TypeTagged<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_map(TypeTaggedVisitor::<V>(PhantomData)).map(TypeTagged)
    }
}

impl<'de, V: BevyTypeTagged> serde::Deserialize<'de> for AnyTagged<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_any(TypeTaggedVisitor::<V>(PhantomData)).map(AnyTagged)
    }
}

struct TypeTaggedVisitor<'de, V: BevyTypeTagged>(PhantomData<&'de V>);

impl<'de, V: BevyTypeTagged> Visitor<'de> for TypeTaggedVisitor<'de, V>  {
    type Value = V;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "externally tagged enum")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de>, {
        let Some(key) = map.next_key::<Cow<str>>()? else {
            return Err(serde::de::Error::custom(
                "expected externally tagged value",
            ));
        };
        if !DESERIALIZE_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        let Some(de_fn) = DESERIALIZE_FUNCTIONS.with(|map| {
            map.get::<V>(&key)
        }) else {
            return Err(serde::de::Error::custom(format!(
                "unregistered type-tag {}", key,
            )));
        };
        map.next_value_seed(DeserializeFnSeed(de_fn, PhantomData))
    }
}

struct DeserializeFnSeed<'de, T: BevyTypeTagged>(DeserializeFn<T>, PhantomData<&'de ()>);

impl<'de, T: BevyTypeTagged> DeserializeSeed<'de> for DeserializeFnSeed<'de, T> {
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: serde::Deserializer<'de> {
        (self.0)(&mut <dyn Deserializer>::erase(deserializer)).map_err(serde::de::Error::custom)
    }
}