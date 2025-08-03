use std::marker::PhantomData;

use ref_cast::RefCast;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

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