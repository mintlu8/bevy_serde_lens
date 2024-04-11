use std::{any::type_name, marker::PhantomData};
use ref_cast::RefCast;
use serde::{de::Visitor, Deserialize, Serialize};
use crate::{BoxError, Convert, FromWorldAccess, SerdeProject, WorldAccess};

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes an [`Option`] containing a [`SerdeProject`] type.
/// 
/// # Example
/// ```
/// # /*
/// // Id implements `SerdeProject` but not `Serialize`
/// #[serde_project("ProjectOption<Option<Id>>")]
/// id: Option<Id>
/// 
/// // `Convert` to another type through the second argument.
/// #[serde_project("ProjectOption<Handle<Image>, PathHandle<Image>>")]
/// image: Option<Handle<Image>>
/// # */
/// ```
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

/// A projection that serializes a [`Vec`] like container of [`SerdeProject`] types.
///
/// The underlying data structure is a [`Vec`], 
/// so you can use `#[serde(skip_serializing_if("Vec::is_empty"))]`.
/// 
/// # Example
/// ```
/// # /*
/// // Id implements `SerdeProject` but not `Serialize`
/// #[serde_project("ProjectVec<Vec<Id>>")]
/// ids: Vec<Id>
/// 
/// // `Convert` to another type through the second argument.
/// #[serde_project("ProjectVec<Vec<Handle<Image>>, PathHandle<Image>>")]
/// images: Vec<Handle<Image>>
/// # */
/// ```
pub type ProjectVec<Iterator, Project = <Iterator as IterVec>::Item> = ProjectVecRaw<Iterator, Project>;

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes a [`Vec`] like container of [`SerdeProject`] types.
///
/// The underlying data structure is a [`Vec`], 
/// so you can use `#[serde(skip_serializing_if("Vec::is_empty"))]`.
pub struct ProjectVecRaw<Iter: IterVec + FromIterator<Iter::Item>, T: Convert<Iter::Item> + SerdeProject>(Iter, PhantomData<T>);

impl<I: IterVec + FromIterator<I::Item>, T: Convert<I::Item> + SerdeProject> Convert<I> for ProjectVecRaw<I, T> {
    fn ser(input: &I) -> &Self {
        ProjectVecRaw::<I, T>::ref_cast(input)
    }

    fn de(self) -> I {
        self.0
    }
}

impl<I: IterVec + FromIterator<I::Item>, T: Convert<I::Item> + SerdeProject> SerdeProject for ProjectVecRaw<I, T> {
    type Ctx = T::Ctx;

    type Ser<'t> = Vec<T::Ser<'t>> where I: 't, T: 't;

    type De<'de> = Vec<T::De<'de>>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        self.0.iter().map(|x| T::ser(x).to_ser(ctx)).collect()
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(
            de.into_iter()
                .map(|de|Ok(T::de(T::from_de(ctx, de)?)))
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


/// Alias for [`IntoIterator`] for `&Self` with `(&Key, &Value)` items.
pub trait IterVec {
    type Item;

    #[doc(hidden)]
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
}

impl<T, K> IterVec for T where for<'t> &'t T: IntoIterator<Item = &'t K> {
    type Item = K;

    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        (&self).into_iter()
    }
}


/// Alias for [`IntoIterator`] for `&Self` with `(&Key, &Value)` items.
pub trait IterTuple {
    type Key;
    type Value;

    #[doc(hidden)]
    fn iter(&self) -> impl Iterator<Item = (&Self::Key, &Self::Value)>;
}

impl<T, K, V> IterTuple for T where for<'t> &'t T: IntoIterator<Item = (&'t K, &'t V)> {
    type Key = K;
    type Value = V;

    fn iter(&self) -> impl Iterator<Item = (&Self::Key, &Self::Value)> {
        (&self).into_iter()
    }
}

/// A projection that serializes a [`Map`] like container of [`SerdeProject`] types.
///
/// The underlying data structure is a [`Map`], 
/// so you can use `#[serde(skip_serializing_if("Map::is_empty"))]`.
/// 
/// # Examples
/// 
/// ```
/// # /*
/// // Id implements `SerdeProject` but not `Serialize`
/// #[serde_project("ProjectMap<HashMap<String, Id>>")]
/// ids: HashMap<String, Id>
/// 
/// // `Convert` key and value to other types through the remaining arguments.
/// #[serde_project("ProjectMap<BTreeMap<String, Handle<Image>>, String, PathHandle<Image>>")]
/// images: BTreeMap<String, Handle<Image>>
/// # */
/// ```
pub type ProjectMap<Map, KeyProject = <Map as IterTuple>::Key, ValueProject = <Map as IterTuple>::Value> 
    = ProjectMapRaw<Map, KeyProject, ValueProject>;

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes a [`Map`] like container of [`SerdeProject`] types.
///
/// The underlying data structure is a [`Map`], 
/// so you can use `#[serde(skip_serializing_if("Map::is_empty"))]`.
pub struct ProjectMapRaw<
    Map: FromIterator<(Map::Key, Map::Value)> + IterTuple, 
    K: Convert<Map::Key> + SerdeProject, 
    V: Convert<Map::Value> + SerdeProject
>(Map, PhantomData<(K, V)>);

impl<I: FromIterator<(I::Key, I::Value)> + IterTuple, K: Convert<I::Key> + SerdeProject, V: Convert<I::Value> + SerdeProject> Convert<I> for ProjectMapRaw<I, K, V> {
    fn ser(input: &I) -> &Self {
        ProjectMapRaw::<I, K, V>::ref_cast(input)
    }

    fn de(self) -> I {
        self.0
    }
}

impl<I: FromIterator<(I::Key, I::Value)> + IterTuple, K: Convert<I::Key> + SerdeProject, V: Convert<I::Value> + SerdeProject> SerdeProject for ProjectMapRaw<I, K, V> {
    type Ctx = WorldAccess;

    type Ser<'t> = Map<K::Ser<'t>, V::Ser<'t>> where I: 't, K: 't, V: 't;

    type De<'de> = Map<K::De<'de>, V::De<'de>>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        self.0.iter().map(|(k ,v)| Ok((
            K::ser(k).to_ser(&<K::Ctx as FromWorldAccess>::from_world(ctx)?)?, 
            V::ser(v).to_ser(&<V::Ctx as FromWorldAccess>::from_world(ctx)?)?, 
        ))).collect()
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(
            de.into_iter()
                .map(|(k, v)|{
                    let k = K::de(K::from_de(&mut <K::Ctx as FromWorldAccess>::from_world_mut(ctx)?, k)?);
                    let v = V::de(V::from_de(&mut <V::Ctx as FromWorldAccess>::from_world_mut(ctx)?, v)?);
                    Ok((k, v))
                })
                .collect::<Result<I, BoxError>>()?, 
            PhantomData
        ))
    }
}