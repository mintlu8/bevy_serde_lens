use std::{
    any::{Any, TypeId},
    borrow::Cow,
    marker::PhantomData,
};

use bevy_ecs::resource::Resource;
use bevy_reflect::TypePath;
use erased_serde::Deserializer;
use rustc_hash::FxHashMap;
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};

use crate::typetagged::ErasedObject;

scoped_tls_hkt::scoped_thread_local! {
    pub(crate) static TYPETAG_SERVER: TypeTagServer
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

pub(crate) struct TypeTaggedVisitor<'de, V: ErasedObject>(pub PhantomData<&'de V>);

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

pub(crate) struct DeserializeFnSeed<'de, T: ErasedObject>(DeserializeFn<T>, PhantomData<&'de ()>);

impl<'de, T: ErasedObject> DeserializeSeed<'de> for DeserializeFnSeed<'de, T> {
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        (self.0)(&mut <dyn Deserializer>::erase(deserializer)).map_err(serde::de::Error::custom)
    }
}
