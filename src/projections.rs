use std::{any::type_name, marker::PhantomData};
use ref_cast::RefCast;
use serde::{de::Visitor, Deserialize, Serialize};
use crate::{BoxError, Convert, FromWorldAccess, SerdeProject, WorldAccess};

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes an [`Option`] containing a [`SerdeProject`] type.
pub struct ProjectOption<From, To: Convert<From> + SerdeProject=From>(Option<From>, PhantomData<To>);

impl<T, U: Convert<T> + SerdeProject> Convert<Option<T>> for ProjectOption<T, U> {
    fn ser(input: &Option<T>) -> &Self {
        ProjectOption::ref_cast(input)
    }

    fn de(self) -> Option<T> {
        self.0
    }
}

impl<T, U: Convert<T> + SerdeProject> SerdeProject for ProjectOption<T, U> {
    type Ctx = U::Ctx;

    type Ser<'t> = Option<U::Ser<'t>> where T: 't, U: 't;

    type De<'de> = Option<U::De<'de>>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        Ok(match &self.0 {
            Some(item) => Some(U::ser(item).to_ser(ctx)?),
            None => None,
        })
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(match de {
            Some(item) => ProjectOption(Some(U::de(U::from_de(ctx, item)?)), PhantomData),
            None => ProjectOption(None, PhantomData),
        })
    }
}

/// Alias for [`ProjectVec`], given type must additionally be [`IntoIterator`].
pub type ProjectVecIter<Iterator, Project = <Iterator as IntoIterator>::Item> = ProjectVec<Iterator, <Iterator as IntoIterator>::Item, Project>;

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes a [`Vec`] like container of [`SerdeProject`] types.
///
/// The underlying data structure is a [`Vec`], 
/// so you can use `#[serde(skip_serializing_if("Vec::is_empty"))]`.
pub struct ProjectVec<I: FromIterator<T>, T, U: Convert<T> + SerdeProject=T>(I, PhantomData<U>) where for<'t> &'t I: IntoIterator<Item = &'t T>;

impl<I: FromIterator<T>, T, U: Convert<T> + SerdeProject> Convert<I> for ProjectVec<I, T, U> where for<'t> &'t I: IntoIterator<Item = &'t T> {
    fn ser(input: &I) -> &Self {
        ProjectVec::<I, T, U>::ref_cast(input)
    }

    fn de(self) -> I {
        self.0
    }
}

impl<I: FromIterator<T>, T, U: Convert<T> + SerdeProject> SerdeProject for ProjectVec<I, T, U> where for<'t> &'t I: IntoIterator<Item = &'t T> {
    type Ctx = U::Ctx;

    type Ser<'t> = Vec<U::Ser<'t>> where I: 't, T: 't, U: 't;

    type De<'de> = Vec<U::De<'de>>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        (&self.0).into_iter().map(|x| U::ser(x).to_ser(ctx)).collect()
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(
            de.into_iter()
                .map(|de|Ok(U::de(U::from_de(ctx, de)?)))
                .collect::<Result<I, BoxError>>()?, 
            PhantomData
        ))
    }
}

/// An internal struct for serializing maps with minimal trait bounds.
#[derive(Debug)]
pub struct Map<K, V>(Vec<(K, V)>);

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Map<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K, V> IntoIterator for Map<K, V> {
    type Item = (K, V);
    
    type IntoIter = std::vec::IntoIter<(K, V)>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V> FromIterator<(K, V)> for Map<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<K: Serialize, V: Serialize> Serialize for Map<K, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0.iter() {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> Deserialize<'de> for Map<K, V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_map(Map(Vec::new()))
    }
}

impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> Visitor<'de> for Map<K, V> {
    type Value = Self;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Map<{}, {}>", type_name::<K>(), type_name::<V>())
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de>, {
        while let Some((k, v)) = map.next_entry()?{
            self.0.push((k, v));
        }
        Ok(self)
    }
}

/// Alias for [`IntoIterator`] with `(Key, Value)` items.
pub trait IterTuple {
    type Key;
    type Value;
}

impl<T, K, V> IterTuple for T where T: IntoIterator<Item = (K, V)> {
    type Key = K;
    type Value = V;
}

/// Alias for [`ProjectMap`], given type must additionally be [`IterTuple`].
pub type ProjectMapIter<Map, KeyProject = <Map as IterTuple>::Key, ValueProject = <Map as IterTuple>::Value> 
    = ProjectMap<Map, <Map as IterTuple>::Key, <Map as IterTuple>::Value, KeyProject, ValueProject>;

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes a [`Map`] like container of [`SerdeProject`] types.
///
/// The underlying data structure is a [`Map`], 
/// so you can use `#[serde(skip_serializing_if("Map::is_empty"))]`.
pub struct ProjectMap<Map: FromIterator<(K, V)>, K, V, K2: Convert<K> + SerdeProject=K, V2: Convert<V> + SerdeProject=V>(Map, PhantomData<(K, V, K2, V2)>) where for<'t> &'t Map: IntoIterator<Item = (&'t K, &'t V)>;

impl<I: FromIterator<(K, V)>, K, V, K2: Convert<K> + SerdeProject, V2: Convert<V> + SerdeProject> Convert<I> for ProjectMap<I, K, V, K2, V2> where for<'t> &'t I: IntoIterator<Item = (&'t K, &'t V)> {
    fn ser(input: &I) -> &Self {
        ProjectMap::<I, K, V, K2, V2>::ref_cast(input)
    }

    fn de(self) -> I {
        self.0
    }
}

impl<I: FromIterator<(K, V)>, K, V, K2: Convert<K> + SerdeProject, V2: Convert<V> + SerdeProject> SerdeProject for ProjectMap<I, K, V, K2, V2> where for<'t> &'t I: IntoIterator<Item = (&'t K, &'t V)> {
    type Ctx = WorldAccess;

    type Ser<'t> = Map<K2::Ser<'t>, V2::Ser<'t>> where I: 't, K: 't, V: 't,  K2: 't, V2: 't;

    type De<'de> = Map<K2::De<'de>, V2::De<'de>>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        (&self.0).into_iter().map(|(k ,v)| Ok((
            K2::ser(k).to_ser(&<K2::Ctx as FromWorldAccess>::from_world(ctx)?)?, 
            V2::ser(v).to_ser(&<V2::Ctx as FromWorldAccess>::from_world(ctx)?)?, 
        ))).collect()
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(
            de.into_iter()
                .map(|(k, v)|{
                    let k = K2::de(K2::from_de(&mut <K2::Ctx as FromWorldAccess>::from_world_mut(ctx)?, k)?);
                    let v = V2::de(V2::from_de(&mut <V2::Ctx as FromWorldAccess>::from_world_mut(ctx)?, v)?);
                    Ok((k, v))
                })
                .collect::<Result<I, BoxError>>()?, 
            PhantomData
        ))
    }
}