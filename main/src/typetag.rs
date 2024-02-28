use std::{borrow::Cow, rc::Rc, sync::Arc};
use bevy_ecs::system::Resource;
use rustc_hash::FxHashMap;
use serde_value::ValueDeserializer;
use serde::{Deserialize, Serialize};
use crate::{BoxError, Error, SerdeProject};

/// A serializable trait object.
pub struct TypeTagged<T: BevyTypeTag>(T);

/// A serializable trait object.
///
/// Implement this on a `dyn T` to work with `Box<dyn T>` 
pub trait BevyTypeTag: 'static {
    fn name(&self) -> &'static str;
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<T> BevyTypeTag for Box<T> where T: BevyTypeTag {
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

impl<T> BevyTypeTag for Rc<T> where T: BevyTypeTag {
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

impl<T> BevyTypeTag for Arc<T> where T: BevyTypeTag {
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

impl<T> BevyTypeTag for Cow<'static, T> where T: BevyTypeTag + ToOwned {
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}

/// A concrete type that implements a [`BevyTypeTag`] trait.
pub trait IntoBevyTypeTag<T: BevyTypeTag> {
    /// Type name, must be unique per type.
    fn name() -> &'static str;
    /// Convert to a [`BevyTypeTag`] type.
    fn into_type_tagged(self) -> T;
}

type DeserializeFn<T> = fn(&mut dyn erased_serde::Deserializer) -> Result<T, erased_serde::Error>;

#[derive(Resource)]
pub struct TypeTagServer<T: BevyTypeTag> {
    functions: FxHashMap<&'static str, DeserializeFn<T>>,
}

impl<T: BevyTypeTag> Default for TypeTagServer<T> {
    fn default() -> Self {
        Self { functions: FxHashMap::default() }
    }
}

impl<T: BevyTypeTag> TypeTagServer<T> {
    pub fn get(&self, name: &str) -> Option<&DeserializeFn<T>>{
        self.functions.get(name)
    }

    pub fn register<A: IntoBevyTypeTag<T>>(&mut self) where for<'de> A: Deserialize<'de>{
        self.functions.insert(A::name(), |de| {
            Ok(A::into_type_tagged(erased_serde::deserialize::<A>(de)?))
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternallyTagged<K, V>(K, V);

impl<T: BevyTypeTag + Send + Sync + 'static> SerdeProject for TypeTagged<T> {
    type Ctx = TypeTagServer<T>;

    type Ser<'t> = ExternallyTagged<&'static str, serde_value::Value>;

    type De<'de> = ExternallyTagged<Cow<'de, str>, serde_value::Value>;

    fn to_ser<'t>(&'t self, _: <Self::Ctx as crate::FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        Ok(ExternallyTagged(
            self.0.name(),
            serde_value::to_value(self.0.as_serialize())
                .map_err(|err| Error::SerializationError(err.to_string()))?
        ))
    }

    fn from_de<'de>(ctx: <Self::Ctx as crate::FromWorldAccess>::Mut<'_>, de: Self::De<'de>) -> Result<Self, BoxError> {
        let f = ctx.get(&de.0).unwrap();
        let de = ValueDeserializer::<serde_value::DeserializerError>::new(de.1);
        Ok(f(&mut <dyn erased_serde::Deserializer>::erase(de)).map(TypeTagged)
            .map_err(|err| Error::DeserializationError(err.to_string()))?)
    }
}