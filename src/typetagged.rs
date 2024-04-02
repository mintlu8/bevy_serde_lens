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
//! on primitives


use std::{any::{Any, TypeId}, borrow::Cow, marker::PhantomData, rc::Rc, sync::Arc};
use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use erased_serde::Deserializer;
use ref_cast::RefCast;
use rustc_hash::FxHashMap;
use scoped_tls::scoped_thread_local;
use serde::{de::{DeserializeOwned, DeserializeSeed, Visitor}, Deserialize, Serialize};
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
pub struct TypeTagged<T: BevyTypeTagged>(pub T);

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
pub struct AnyTagged<T: BevyTypeTagged>(pub T);

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
pub trait BevyTypeTagged: 'static {
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

mod sealed {
    pub trait Sealed<E> {}
}

use sealed::Sealed;

pub trait DeserializeAnyFn<T, E>: Sealed<E> {
    fn register(self, server: &mut DeserializeAnyServer);
}

macro_rules! impl_de_any_fn {
    ($($in: ty, $out: ident, $name: ident);*;) => {
        $(
            type $name<T> = Box<dyn Fn($in) -> Result<T, String> + Send + Sync + 'static>;
            impl<T: BevyTypeTagged, F> Sealed<$in> for F where F: Fn($in) -> Result<T, String> + Send + Sync + 'static{
            }
            impl<T: BevyTypeTagged, F> DeserializeAnyFn<T, $in> for F where F: Fn($in) -> Result<T, String> + Send + Sync + 'static{
                fn register(self, server: &mut DeserializeAnyServer) {
                    let id = TypeId::of::<T>();
                    server.$out.insert(id, Box::new(Box::new(self) as $name<T>));
                }
            }
        )*
    };
}

impl_de_any_fn!(
    bool, deserialize_bool, DeserializeBoolFn;
    i64, deserialize_int, DeserializeIntFn;
    u64, deserialize_uint, DeserializeUIntFn;
    f64, deserialize_float, DeserializeFloatFn;
    char, deserialize_char, DeserializeCharFn;
    &str, deserialize_str, DeserializeStrFn;
    &[u8], deserialize_bytes, DeserializeBytesFn;
);

type DeserializeUnitFn<T> = Box<dyn Fn() -> Result<T, String> + Send + Sync + 'static>;

impl<T: 'static, F> Sealed<()> for F where F: Fn() -> Result<T, String> + Send + Sync + 'static{}

impl<T: 'static, F> DeserializeAnyFn<T, ()> for F where F: Fn() -> Result<T, String> + Send + Sync + 'static{
    fn register(self, server: &mut DeserializeAnyServer) {
        let id = TypeId::of::<T>();
        server.deserialize_unit.insert(id, Box::new(Box::new(self) as DeserializeUnitFn<T>));
    }
}

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
    }

    pub fn register<T: BevyTypeTagged, Marker>(&mut self, f: impl DeserializeAnyFn<T, Marker>) {
        f.register(self)
    }

    pub fn get_unit<T: BevyTypeTagged>(&self) -> Option<&DeserializeUnitFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_unit.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_bool<T: BevyTypeTagged>(&self) -> Option<&DeserializeBoolFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_bool.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_int<T: BevyTypeTagged>(&self) -> Option<&DeserializeIntFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_int.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_uint<T: BevyTypeTagged>(&self) -> Option<&DeserializeUIntFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_uint.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_float<T: 'static>(&self) -> Option<&DeserializeFloatFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_float.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_char<T: 'static>(&self) -> Option<&DeserializeCharFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_char.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_str<T: 'static>(&self) -> Option<&DeserializeStrFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_str.get(&id).map(|f| f.downcast_ref().unwrap())
    }

    pub fn get_bytes<T: 'static>(&self) -> Option<&DeserializeBytesFn<T>>{
        let id = TypeId::of::<T>();
        self.deserialize_bytes.get(&id).map(|f| f.downcast_ref().unwrap())
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

    fn visit_unit<E>(self) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_unit::<V>().map(|f| f())}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_unit unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_bool::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_bool unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_int::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_i64 unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_uint::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_u64 unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_float::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_f64 unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_char::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_char unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_str::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_str unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }
    
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: serde::de::Error {
        if !DESERIALIZE_ANY_FUNCTIONS.is_set(){
            return Err(serde::de::Error::custom(
                "cannot deserialize `TypeTagged` value outside the `save` context.",
            ));
        }
        match DESERIALIZE_ANY_FUNCTIONS.with(|map| { map.get_bytes::<V>().map(|f| f(v))}) {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) => Err(serde::de::Error::custom(error)),
            None => Err(serde::de::Error::custom(format!(
                "deserialize_bytes unregistered for {}", std::any::type_name::<V>(),
            ))),
        }
    }
    
}

struct DeserializeFnSeed<'de, T: BevyTypeTagged>(DeserializeFn<T>, PhantomData<&'de ()>);

impl<'de, T: BevyTypeTagged> DeserializeSeed<'de> for DeserializeFnSeed<'de, T> {
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: serde::Deserializer<'de> {
        (self.0)(&mut <dyn Deserializer>::erase(deserializer)).map_err(serde::de::Error::custom)
    }
}

impl<T> BevyTypeTagged for Box<T> where T: BevyTypeTagged + ?Sized {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

impl<T> BevyTypeTagged for Rc<T> where T: BevyTypeTagged + ?Sized {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

impl<T> BevyTypeTagged for Arc<T> where T: BevyTypeTagged + ?Sized {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

impl<T> BevyTypeTagged for Cow<'static, T> where T: BevyTypeTagged + ToOwned + ?Sized, T::Owned: Send + Sync + 'static {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

/// A basic trait object that satisfies [`BevyTypeTagged`]. 
/// 
/// All [`TypePath`] and [`Serialize`] types automatically implements this.
pub trait TaggedAny: Any + Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<T> TaggedAny for T where T: Serialize + TypePath + Send + Sync + 'static {
    fn name(&self) -> &'static str {
        T::short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

impl BevyTypeTagged for dyn TaggedAny {
    fn name(&self) -> impl AsRef<str> {
        TaggedAny::name(self)
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        TaggedAny::as_serialize(self)
    }
}

impl<T> FromTypeTagged<T> for Box<dyn TaggedAny> where T: Serialize + DeserializeOwned + TypePath + Send + Sync + 'static {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Box::new(item)
    }
}

impl<T> FromTypeTagged<T> for Rc<dyn TaggedAny> where T: Serialize + DeserializeOwned + TypePath + Send + Sync + 'static {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Rc::new(item)
    }
}

impl<T> FromTypeTagged<T> for Arc<dyn TaggedAny> where T: Serialize + DeserializeOwned + TypePath + Send + Sync + 'static {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Arc::new(item)
    }
}


/// A primitive equivalent to `Cow` on a typetagged [`BevyTypeTagged`].
/// `CowTypeTagged<dyn Trait>` is equivalent to `TypeTagged<Box<dyn Trait>>`.
/// 
/// This type is [`Serialize`] and [`Deserialize`], similar to [`TypeTagged`],
/// but can serialize a borrowed `BevyTypeTagged` trait object.
/// This is useful for implementing `SerdeProject`.
#[derive(Debug)]
pub enum CowTypeTagged<'t, T: BevyTypeTagged>{
    Borrowed(&'t T),
    Owned(Box<T>),
}

impl<T: BevyTypeTagged> CowTypeTagged<'_, T> {
    pub fn into_owned(self) -> Box<T> {
        match self {
            CowTypeTagged::Borrowed(_) => panic!("deserialize will only return owned value."),
            CowTypeTagged::Owned(v) => v,
        }
    }
}

impl<T: BevyTypeTagged> Serialize for CowTypeTagged<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeMap;
        let item = match self {
            CowTypeTagged::Borrowed(item) => item,
            CowTypeTagged::Owned(item) => item.as_ref(),
        };

        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(item.name().as_ref(), item.as_serialize())?;
        map.end()
    }
}

impl<'de, T: BevyTypeTagged> Deserialize<'de> for CowTypeTagged<'_, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        Ok(CowTypeTagged::Owned(TypeTagged::<Box<T>>::deserialize(deserializer)?.0))
    }
}


/// A primitive equivalent to `Cow` on a typetagged [`BevyTypeTagged`].
/// `CowAnyTagged<dyn Trait>` is equivalent to `AnyTagged<Box<dyn Trait>>`.
/// 
/// This type is [`Serialize`] and [`Deserialize`], similar to [`AnyTagged`],
/// but can serialize a borrowed `BevyTypeTagged` trait object.
/// This is useful for implementing `SerdeProject`.
#[derive(Debug)]
pub enum CowAnyTagged<'t, T: BevyTypeTagged>{
    Borrowed(&'t T),
    Owned(Box<T>),
}

impl<T: BevyTypeTagged> CowAnyTagged<'_, T> {
    pub fn into_owned(self) -> Box<T> {
        match self {
            CowAnyTagged::Borrowed(_) => panic!("deserialize will only return owned value."),
            CowAnyTagged::Owned(v) => v,
        }
    }
}

impl<T: BevyTypeTagged> Serialize for CowAnyTagged<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeMap;
        let item = match self {
            CowAnyTagged::Borrowed(item) => item,
            CowAnyTagged::Owned(item) => item.as_ref(),
        };

        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(item.name().as_ref(), item.as_serialize())?;
        map.end()
    }
}

impl<'de, T: BevyTypeTagged> Deserialize<'de> for CowAnyTagged<'_, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        Ok(CowAnyTagged::Owned(AnyTagged::<Box<T>>::deserialize(deserializer)?.0))
    }
}
