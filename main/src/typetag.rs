use std::borrow::Cow;
use bevy_ecs::system::Resource;
use rustc_hash::FxHashMap;
use serde_value::ValueDeserializer;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::{BoxError, Error, SerdeProject};

/// A serializable trait object.
pub struct TypeTagged<T: BevyTypeTagged>(T);

/// A serializable trait object.
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
/// pub trait Stat: erased_serde::Serialize {
///     fn name(&self) -> &'static str {
///         std::any::type_name()
///     }
///
///     fn as_serialize(&self) -> &dyn erased_serde::Serialize {
///         self
///     }
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
/// fn main() {
///     ..
///     app.register_typetag::<Box<dyn<Stat>>, MyStat>   
/// }
/// ```
pub trait BevyTypeTagged: 'static {
    /// Returns the type name of the implementor.
    fn name(&self) -> &'static str;
    /// Returns the untagged inner value of the implementor.
    ///
    /// # Note
    ///
    /// If you used the actual `typetag` crate on your trait, be sure to use
    /// return a reference to the inner value instead of `dyn YourTrait`.
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

/// A concrete type that implements a [`BevyTypeTag`] trait.
pub trait IntoTypeTagged<T: BevyTypeTagged>: DeserializeOwned {
    /// Type name, must be unique per type.
    fn name() -> &'static str;
    /// Convert to a [`BevyTypeTag`] type.
    fn into_type_tagged(self) -> T;
}

type DeserializeFn<T> = fn(&mut dyn erased_serde::Deserializer) -> Result<T, erased_serde::Error>;

/// A [`Resource`] that stores registered deserialization functions.
#[derive(Resource)]
pub struct TypeTagServer<T: BevyTypeTagged> {
    functions: FxHashMap<&'static str, DeserializeFn<T>>,
}

impl<T: BevyTypeTagged> Default for TypeTagServer<T> {
    fn default() -> Self {
        Self { functions: FxHashMap::default() }
    }
}

impl<T: BevyTypeTagged> TypeTagServer<T> {
    pub fn get(&self, name: &str) -> Option<&DeserializeFn<T>>{
        self.functions.get(name)
    }

    pub fn clear(&mut self) {
        self.functions.clear()
    }

    pub fn register<A: IntoTypeTagged<T>>(&mut self) {
        self.functions.insert(A::name(), |de| {
            Ok(A::into_type_tagged(erased_serde::deserialize::<A>(de)?))
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternallyTagged<K, V>(K, V);

impl<T: BevyTypeTagged + Send + Sync + 'static> SerdeProject for TypeTagged<T> {
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