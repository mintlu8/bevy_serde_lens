use bevy_app::App;
use bevy_ecs::query::QueryState;
use bevy_ecs::system::Resource;
use bevy_ecs::{entity::Entity, world::World};
use bevy_hierarchy::BuildWorldChildren;
use std::cell::RefCell;
use std::sync::Mutex;
use std::{borrow::Cow, marker::PhantomData};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;

use crate::typetagged::{scoped, scoped_any, BevyTypeTagged, DeserializeAnyFn, DeserializeAnyServer, IntoTypeTagged, TypeTagServer};
use crate::{from_world, from_world_mut, BevyObject, BindBevyObject, Named, SerdeProject, WorldUtil};

#[allow(unused)]
use crate::batch;

/// A [`Serialize`] type from a [`World`] reference and a [`SaveLoad`] type.
pub struct SerializeLens<'t, S: SaveLoad>(Mutex<&'t mut World>, PhantomData<S>);
 
impl<T: SaveLoad> Serialize for SerializeLens<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.lock().unwrap().save::<T, S>(serializer)
    }
}

/// A [`DeserializeSeed`] type from a [`World`] reference and a [`SaveLoad`] type.
pub struct DeserializeLens<'t, S: SaveLoad>(&'t mut World, PhantomData<S>);

impl<'de, T: SaveLoad> DeserializeSeed<'de> for DeserializeLens<'de, T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        self.0.load::<T, D>(deserializer)
    }
}


/// Extension methods on [`World`].
pub trait WorldExtension {
    /// Save a [`BindBevyObject`] type or a group created by [`batch!`].
    ///
    /// # What's a [`Serializer`]?
    ///
    /// Most `serde` frontends provide a serializer, like `serde_json::Serializer`.
    /// They typically wrap a [`std::io::Write`] and write to that stream.
    fn save<T: SaveLoad, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error>;
    /// Load a [`BindBevyObject`] type or a group created by [`batch!`].
    ///
    /// # What's a [`Deserializer`]?
    ///
    /// Most `serde` frontends provide a serializer, like `serde_json::Deserializer`.
    /// They typically wrap a [`std::io::Read`] and read from that stream.
    fn load<'de, T: SaveLoad, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error>;
    /// Create a [`Serialize`] type from a [`World`] and a [`SaveLoad`] type.
    fn as_serialize_lens<S: SaveLoad>(&mut self) -> SerializeLens<S>;
    /// Create a [`DeserializeSeed`] type from a [`World`] and a [`SaveLoad`] type.
    fn as_deserialize_lens<S: SaveLoad>(&mut self) -> DeserializeLens<S>;
    /// Despawn all entities in a [`BindBevyObject`] type or a group created by [`batch!`] recursively.
    fn despawn_bound_objects<T: SaveLoad>(&mut self);
    /// Register a type that can be deserialized dynamically.
    fn register_typetag<A: BevyTypeTagged, B: IntoTypeTagged<A>>(&mut self);
    /// Register a type that can be deserialized dynamically from a primitive.
    /// 
    /// Accepts a `Fn(T) -> Result<Out, String>` where T is `()`, `bool`, `i64`, `u64`, `f64`, `char`, `&str` or `&[u8]`.
    /// 
    /// # Example 
    /// ```
    /// // deserialize number as the default attacking type
    /// app.register_deserialize_any(|x: i64| Ok(DefaultAttack::new(x as i32)));
    /// ```
    fn register_deserialize_any<T: BevyTypeTagged, O>(&mut self, f: impl DeserializeAnyFn<T, O>);
}

impl WorldExtension for World {
    fn save<T: SaveLoad, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error> {
        T::save(self, serializer)
    }

    fn load<'de, T: SaveLoad, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error> {
        macro_rules! inner {
            () => {
                if let Some(server) = self.remove_resource::<DeserializeAnyServer>() {
                    let result = scoped_any(&server, ||T::load(self, deserializer));
                    self.insert_resource(server);
                    result
                } else {
                    T::load(self, deserializer)
                }
            };
        }
        if let Some(server) = self.remove_resource::<TypeTagServer>() {
            let result = scoped(&server, ||inner!());
            self.insert_resource(server);
            result
        } else {
            inner!()
        }
    }

    fn as_serialize_lens<S: SaveLoad>(&mut self) -> SerializeLens<S> {
        SerializeLens(Mutex::new(self), PhantomData)
    }

    fn as_deserialize_lens<S: SaveLoad>(&mut self) -> DeserializeLens<S> {
        DeserializeLens(self, PhantomData)
    }

    fn despawn_bound_objects<T: SaveLoad>(&mut self){
        T::despawn(self)
    }

    fn register_typetag<A: BevyTypeTagged, B: IntoTypeTagged<A>>(&mut self){
        let mut server = self.get_resource_or_insert_with(TypeTagServer::default);
        server.register::<A, B>()
    }

    fn register_deserialize_any<T: BevyTypeTagged, O>(&mut self, f: impl DeserializeAnyFn<T, O>) {
        let mut server = self.get_resource_or_insert_with(DeserializeAnyServer::default);
        server.register::<T, O>(f)
    }
}

impl WorldExtension for App {
    fn save<T: SaveLoad, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error> {
        self.world.save::<T, S>(serializer)
    }

    fn load<'de, T: SaveLoad, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error> {
        self.world.load::<T, D>(deserializer)
    }

    fn as_serialize_lens<S: SaveLoad>(&mut self) -> SerializeLens<S> {
        self.world.as_serialize_lens()
    }

    fn as_deserialize_lens<S: SaveLoad>(&mut self) -> DeserializeLens<S> {
        self.world.as_deserialize_lens()
    }

    fn despawn_bound_objects<T: SaveLoad>(&mut self){
        self.world.despawn_bound_objects::<T>()
    }

    fn register_typetag<A: BevyTypeTagged, B: IntoTypeTagged<A>>(&mut self){
        self.world.register_typetag::<A, B>()
    }

    fn register_deserialize_any<T: BevyTypeTagged, O>(&mut self, f: impl DeserializeAnyFn<T, O>) {
        self.world.register_deserialize_any::<T, O>(f)
    }
}

/// A batch save/load type.
pub trait SaveLoad: Sized {
    const COUNT: usize;
    type First: DeserializeFromMap;
    type Remaining: SaveLoad;

    fn save<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error>;
    fn load<'de, D: Deserializer<'de>>(world: &mut World, deserializer: D) -> Result<(), D::Error>;
    fn save_map<S: SerializeMap>(world: &mut World, serializer: &mut S) -> Result<(), S::Error>;
    fn despawn(world: &mut World);
}

/// Bind a [`Resource`] to be serialized.
/// 
/// Requires the resource to implement [`Named`].
pub struct BindResource<R: Resource + Named>(R);

pub trait DeserializeFromMap {
    fn name() -> &'static str;

    fn visit_map_single<'de, A>(world: &mut World, map: &mut A) -> Result<(), A::Error> where A: MapAccess<'de>;
}

impl<T> DeserializeFromMap for T where T: BindBevyObject {
    fn name() -> &'static str {
        <T as BindBevyObject>::name()
    }

    fn visit_map_single<'de, A>(world: &mut World, map: &mut A) -> Result<(), A::Error> where A: MapAccess<'de> {
        let root = T::get_root(world);
        map.next_value_seed(&mut SingleComponentSeed {
            world,
            root,
            p: PhantomData::<T>,
        })?;
        Ok(())
    }
}

impl<R: Resource + SerdeProject + Named> DeserializeFromMap for BindResource<R> {
    fn name() -> &'static str {
        R::name()
    }

    fn visit_map_single<'de, A>(world: &mut World, map: &mut A) -> Result<(), A::Error> where A: MapAccess<'de> {
        let item = map.next_value::<R::De::<'de>>()?;
        let res = (||R::from_de(&mut from_world_mut::<R>(world)?, item))()
            .map_err(serde::de::Error::custom)?;
        world.insert_resource(res);
        Ok(())
    }
}

impl<R: Resource + SerdeProject + Named> SaveLoad for BindResource<R> {
    const COUNT: usize = 1;
    type First = BindResource<R>;
    type Remaining = Self;

    fn save<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        (|| {
            world.resource_ok::<R>()?.to_ser(&from_world::<R>(world)?)
        })()
            .map_err(serde::ser::Error::custom)
            .and_then(|x| x.serialize(serializer))
    }

    fn load<'de, D: Deserializer<'de>>(world: &mut World, deserializer: D) -> Result<(), D::Error> {
        let item = R::De::deserialize(deserializer)?;
        let res = (||R::from_de(&mut from_world_mut::<R>(world)?, item))()
            .map_err(serde::de::Error::custom)?;
        world.insert_resource(res);
        Ok(())
    }

    fn save_map<S: SerializeMap>(world: &mut World, serializer: &mut S) -> Result<(), S::Error> {
        serializer.serialize_entry(
            R::name(),
            &(|| {
                world.resource_ok::<R>()?.to_ser(&from_world::<R>(world)?)
            })().map_err(serde::ser::Error::custom)?
        )
    }

    fn despawn(world: &mut World) {
        world.remove_resource::<R>();
    }
}

impl<T> SaveLoad for T where T: BindBevyObject {
    const COUNT: usize = 1;

    type First = Self;
    type Remaining = Self;

    fn save<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        let mut err = None;
        let mut query = world.query_filtered::<Entity, T::Filter>();
        let iter = query
            .iter(world)
            .filter_map(|entity| <T::BevyObject as BevyObject>::to_ser(world, entity).transpose())
            .map_while(|result| {
                match result {
                    Ok(some) => Some(some),
                    Err(e) => { 
                        err = Some(e);
                        None
                    },
                }
            });
        let ser = if serializer.is_human_readable() || Some(iter.size_hint().0) == iter.size_hint().1 {
            serializer.collect_seq(iter)
        } else {
            iter.into_iter().collect::<Vec<_>>().serialize(serializer)
        };
        if let Some(err) = err {
            Err(serde::ser::Error::custom(err))
        } else {
            ser
        }
    }

    fn load<'de, D: Deserializer<'de>>(world: &mut World, deserializer: D) -> Result<(), D::Error> {
        let root = T::get_root(world);
        deserializer.deserialize_seq(SingleComponentVisitor::<T>{
            world,
            root,
            p: PhantomData,
        })
    }

    fn save_map<S: SerializeMap>(world: &mut World, serializer: &mut S) -> Result<(), S::Error>{
        serializer.serialize_key(Self::name())?;
        let state = world.query_filtered::<Entity, T::Filter>();
        serializer.serialize_value(&SerializeSeed {
            world,
            state: RefCell::new(state),
            p: PhantomData::<T>
        })?;
        Ok(())
    }

    fn despawn(world: &mut World) {
        let mut query = world.query_filtered::<Entity, T::Filter>();
        let queue = query.iter(world).collect::<Vec<_>>();
        for entity in queue {
            bevy_hierarchy::despawn_with_children_recursive(world, entity);
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Join<A, B>(PhantomData<(A, B)>);

struct SerializeSeed<'t, T: BindBevyObject> {
    world: &'t World,
    state: RefCell<QueryState<Entity, T::Filter>>,
    p: PhantomData<T>
}

impl<'t, T: BindBevyObject> Serialize for SerializeSeed<'t, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut err = None;
        let ser = serializer.collect_seq(
            self.state.borrow_mut()
                .iter(self.world)
                .filter_map(|entity| <T::BevyObject as BevyObject>::to_ser(self.world, entity).transpose())
                .map_while(|result| {
                    match result {
                        Ok(some) => Some(some),
                        Err(e) => { 
                            err = Some(e);
                            None
                        },
                    }
                } )
        );
        if let Some(err) = err {
            Err(serde::ser::Error::custom(err))
        } else {
            ser
        }
    }
}

impl<A, B> SaveLoad for Join<A, B> where A: SaveLoad + DeserializeFromMap, B: SaveLoad {
    const COUNT: usize = A::COUNT + B::COUNT;

    type First = A;
    type Remaining = B;

    fn save<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(Self::COUNT))?;
        Self::save_map(world, &mut map)?;
        map.end()
    }

    fn load<'de, D: Deserializer<'de>>(world: &mut World, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(MultiComponentVisitor::<Self>{
            world,
            p: PhantomData,
        })
    }

    fn save_map<S: SerializeMap>(world: &mut World, serializer: &mut S) -> Result<(), S::Error>{
        A::save_map(world, serializer)?;
        B::save_map(world, serializer)?;
        Ok(())
    }

    fn despawn(world: &mut World) {
        A::despawn(world);
        B::despawn(world);
    }
}

pub struct SingleComponentVisitor<'t, T: BindBevyObject> {
    world: &'t mut World,
    root: Option<Entity>,
    p: PhantomData<T>
}

impl<'a, 'de, T: BindBevyObject> Visitor<'de> for SingleComponentVisitor<'a, T>{
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "array of type {}", std::any::type_name::<T>())
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de>, {
        loop {
            if seq.next_element_seed(&mut self)?.is_none(){
                return Ok(());
            }
        }
    }
}

impl<'a, 'de, T: BindBevyObject> DeserializeSeed<'de> for &'_ mut SingleComponentVisitor<'a, T>{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        let de = <T::BevyObject as BevyObject>::De::deserialize(deserializer)?;
        let entity = self.world.spawn_empty().id();
        <T::BevyObject as BevyObject>::from_de(self.world, entity, de)
            .map_err(serde::de::Error::custom)?;
        if let Some(root) = self.root {
            self.world.entity_mut(root).add_child(entity);
        }
        Ok(())
    }
}


pub struct SingleComponentSeed<'t, T: BindBevyObject> {
    world: &'t mut World,
    root: Option<Entity>,
    p: PhantomData<T>
}


impl<'a, 'de, T: BindBevyObject> DeserializeSeed<'de> for &'_ mut SingleComponentSeed<'a, T>{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_seq(SingleComponentVisitor {
            world: self.world,
            root: self.root,
            p: self.p
        })
    }
}
pub struct MultiComponentVisitor<'t, T: SaveLoad> {
    world: &'t mut World,
    p: PhantomData<T>
}

impl<'t, 'de, T: SaveLoad> MultiComponentVisitor<'t, T> {
    fn visit_map_single<A>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> where A: MapAccess<'de>, {
        if T::First::name() == key {
            <T::First as DeserializeFromMap>::visit_map_single(self.world, map)?;
        } else if T::COUNT > 1 {
            MultiComponentVisitor::<T::Remaining> {
                world: self.world,
                p: PhantomData
            }.visit_map_single(key, map)?;
        } else {
            map.next_value::<IgnoredAny>()?;
        }
        Ok(())
    }
}

impl<'a, 'de, T: SaveLoad> Visitor<'de> for MultiComponentVisitor<'a, T>{
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Expected map.")
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de>, {
        while let Some(name) = map.next_key::<Cow<str>>()?{
            self.visit_map_single(&name, &mut map)?;
        }
        Ok(())
    }
}
