use crate::{
    entity_scope, ser_scope, BevyObject, Root, SerializeNonSend, SerializeResource, ZstInit,
};
use bevy_ecs::{entity::Entity, resource::Resource, world::World};
use bevy_reflect::TypePath;
use serde::{
    de::{DeserializeOwned, MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{borrow::Cow, cell::RefCell, marker::PhantomData};

/// A batch serialization type.
pub trait BatchSerialization {
    type De: DeserializeOwned + ZstInit;
    const LEN: usize;
    fn despawn(world: &mut World);
    fn serialize<S: Serializer>(world: &mut World, s: S) -> Result<S::Ok, S::Error>;
    fn save_map<S: SerializeMap>(serializer: &mut S, world: &mut World) -> Result<(), S::Error>;
    fn deserialize_map<'de, M>(name: &str, map: &mut M) -> Result<(), M::Error>
    where
        M: MapAccess<'de>;
}

/// A Single item in [`BatchSerialization`].
pub trait SerializeWorld {
    type De: DeserializeOwned + ZstInit;
    fn name() -> &'static str;
    fn serialize<S: Serializer>(world: &mut World, s: S) -> Result<S::Ok, S::Error>;
    fn despawn(world: &mut World);
}

impl<T> BatchSerialization for T
where
    T: SerializeWorld,
{
    type De = T::De;

    const LEN: usize = 1;

    fn despawn(world: &mut World) {
        <T as SerializeWorld>::despawn(world)
    }

    fn serialize<S: Serializer>(world: &mut World, s: S) -> Result<S::Ok, S::Error> {
        <T as SerializeWorld>::serialize(world, s)
    }

    fn save_map<S: SerializeMap>(serializer: &mut S, world: &mut World) -> Result<(), S::Error> {
        serializer.serialize_entry(
            Self::name(),
            &SerializeWorldLens {
                world: RefCell::new(world),
                p: PhantomData::<T>,
            },
        )
    }

    fn deserialize_map<'de, M>(name: &str, map: &mut M) -> Result<(), M::Error>
    where
        M: MapAccess<'de>,
    {
        if name == Self::name() {
            map.next_value::<T::De>()?;
            Ok(())
        } else {
            Err(serde::de::Error::custom(format!(
                "Unknown type name {name}."
            )))
        }
    }
}

pub(crate) struct SerializeWorldLens<'t, S: SerializeWorld> {
    pub(crate) world: RefCell<&'t mut World>,
    pub(crate) p: PhantomData<S>,
}

impl<T: SerializeWorld> serde::Serialize for SerializeWorldLens<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        T::serialize(*self.world.borrow_mut(), serializer)
    }
}

impl<T> SerializeWorld for T
where
    T: BevyObject,
{
    type De = Root<T>;

    fn name() -> &'static str {
        <T as BevyObject>::name()
    }

    fn serialize<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        if T::IS_QUERY {
            let mut query = world.query_filtered::<T::Data, T::Filter>();
            ser_scope(world, || {
                serializer.collect_seq(query.iter(world).map(T::into_ser))
            })
        } else {
            use serde::ser::SerializeSeq;
            let mut query = world.query_filtered::<Entity, T::Filter>();
            let mut seq = serializer.serialize_seq(Some(query.iter(world).count()))?;
            for entity in query.iter(world) {
                ser_scope(world, || {
                    entity_scope(entity, || seq.serialize_element(&T::init()))
                })?;
            }
            seq.end()
        }
    }

    fn despawn(world: &mut World) {
        let mut query = world.query_filtered::<Entity, T::Filter>();
        let queue = query.iter(world).collect::<Vec<_>>();
        for entity in queue {
            let _ = world.despawn(entity);
        }
    }
}

impl<T> SerializeWorld for SerializeResource<T>
where
    T: Resource + Serialize + DeserializeOwned + TypePath,
{
    type De = Self;
    fn name() -> &'static str {
        T::short_type_path()
    }

    fn serialize<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        ser_scope(world, || Self::init().serialize(serializer))
    }

    fn despawn(world: &mut World) {
        world.remove_resource::<T>();
    }
}

impl<T> SerializeWorld for SerializeNonSend<T>
where
    T: Serialize + DeserializeOwned + TypePath + 'static,
{
    type De = Self;
    fn name() -> &'static str {
        T::short_type_path()
    }

    fn serialize<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        ser_scope(world, || Self::init().serialize(serializer))
    }

    fn despawn(world: &mut World) {
        world.remove_non_send_resource::<T>();
    }
}

/// Join two [`BatchSerialization`] types.
#[derive(Debug, Clone, Copy, Default)]
pub struct Join<A, B>(PhantomData<(A, B)>);

impl<A, B> ZstInit for Join<A, B> {
    fn init() -> Self {
        Join(PhantomData)
    }
}

impl<A, B> BatchSerialization for Join<A, B>
where
    A: SerializeWorld,
    B: BatchSerialization,
{
    type De = Self;
    const LEN: usize = B::LEN + 1;
    fn despawn(world: &mut World) {
        A::despawn(world);
        B::despawn(world);
    }

    fn save_map<S: SerializeMap>(serializer: &mut S, world: &mut World) -> Result<(), S::Error> {
        serializer.serialize_entry(
            A::name(),
            &SerializeWorldLens {
                world: RefCell::new(world),
                p: PhantomData::<A>,
            },
        )?;
        B::save_map(serializer, world)
    }

    fn serialize<S: Serializer>(world: &mut World, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(Self::LEN))?;
        Self::save_map(&mut map, world)?;
        B::save_map(&mut map, world)?;
        map.end()
    }

    fn deserialize_map<'de, M>(name: &str, map: &mut M) -> Result<(), M::Error>
    where
        M: MapAccess<'de>,
    {
        if name == A::name() {
            map.next_value::<A::De>()?;
        } else {
            B::deserialize_map(name, map)?;
        }
        Ok(())
    }
}

impl<'de, A, B> Deserialize<'de> for Join<A, B>
where
    A: SerializeWorld,
    B: BatchSerialization,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(Join::<A, B>(PhantomData))
    }
}

impl<'de, A, B> Visitor<'de> for Join<A, B>
where
    A: SerializeWorld,
    B: BatchSerialization,
{
    type Value = Join<A, B>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("map of types")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<Cow<str>>()? {
            if key.as_ref() == A::name() {
                map.next_value::<A::De>()?;
            } else {
                B::deserialize_map(key.as_ref(), &mut map)?;
            }
            //Self::deserialize_map(&key, &mut map)?
        }
        Ok(Join(PhantomData))
    }
}
