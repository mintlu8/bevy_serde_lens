use bevy_app::App;
use bevy_ecs::query::QueryState;
use bevy_ecs::{entity::Entity, query::With, world::World};
use bevy_hierarchy::BuildWorldChildren;
use std::cell::RefCell;
use std::{borrow::Cow, marker::PhantomData};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;

use crate::typetagged::{IntoTypeTagged, BevyTypeTagged, TypeTagServer};
use crate::{BindBevyObject, BevyObject};

#[allow(unused)]
use crate::batch;

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
    /// Despawn all entities in a [`BindBevyObject`] type or a group created by [`batch!`] recursively.
    fn despawn_bound_objects<T: SaveLoad>(&mut self);
    /// Register a type that can be deserialized dynamically.
    fn register_typetag<A: BevyTypeTagged, B: IntoTypeTagged<A>>(&mut self);
}

impl WorldExtension for World {
    fn save<T: SaveLoad, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error> {
        T::save(self, serializer)
    }

    fn load<'de, T: SaveLoad, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error> {
        T::load(self, deserializer)
    }

    fn despawn_bound_objects<T: SaveLoad>(&mut self){
        T::despawn(self)
    }

    fn register_typetag<A: BevyTypeTagged, B: IntoTypeTagged<A>>(&mut self){
        let mut server = self.get_resource_or_insert_with(TypeTagServer::<A>::default);
        server.register::<B>()
    }
}

impl WorldExtension for App {
    fn save<T: SaveLoad, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error> {
        T::save(&mut self.world, serializer)
    }

    fn load<'de, T: SaveLoad, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error> {
        T::load(&mut self.world, deserializer)
    }

    fn despawn_bound_objects<T: SaveLoad>(&mut self){
        T::despawn(&mut self.world)
    }

    fn register_typetag<A: BevyTypeTagged, B: IntoTypeTagged<A>>(&mut self){
        let mut server = self.world.get_resource_or_insert_with(TypeTagServer::<A>::default);
        server.register::<B>()
    }
}


pub trait SaveLoad: Sized {
    const COUNT: usize;
    type First: BindBevyObject;
    type Remaining: SaveLoad;

    fn save<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error>;
    fn load<'de, D: Deserializer<'de>>(world: &mut World, deserializer: D) -> Result<(), D::Error>;
    fn save_map<S: SerializeMap>(world: &mut World, serializer: &mut S) -> Result<(), S::Error>;
    fn despawn(world: &mut World);

}

impl<T> SaveLoad for T where T: BindBevyObject {
    const COUNT: usize = 1;

    type First = Self;
    type Remaining = Self;

    fn save<S: Serializer>(world: &mut World, serializer: S) -> Result<S::Ok, S::Error> {
        let mut err = None;
        let ser = serializer.collect_seq(
            world.query_filtered::<Entity, With<T>>()
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
                } )
        );
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
        let state = world.query_filtered::<Entity, With<Self>>();
        serializer.serialize_value(&SerializeSeed {
            world,
            state: RefCell::new(state),
            p: PhantomData
        })?;
        Ok(())
    }

    fn despawn(world: &mut World) {
        let mut query = world.query_filtered::<Entity, With<T>>();
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
    state: RefCell<QueryState<Entity, With<T>>>,
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

impl<A, B> SaveLoad for Join<A, B> where A: BindBevyObject, B: SaveLoad {
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
        let entity = self.world.spawn(()).id();
        <T::BevyObject as BevyObject>::from_de(self.world, entity, de)
            .map_err(serde::de::Error::custom)?;
        if let Some(root) = self.root {
            self.world.entity_mut(root).add_child(entity);
        }
        Ok(())
    }
}

pub struct MultiComponentVisitor<'t, T: SaveLoad> {
    world: &'t mut World,
    p: PhantomData<T>
}

impl<'t, 'de, T: SaveLoad> MultiComponentVisitor<'t, T> {
    fn visit_map_single<A>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> where A: MapAccess<'de>, {
        if <T::First as BindBevyObject>::name() == key {
            let root = <T::First as BindBevyObject>::get_root(self.world);
            map.next_value_seed(&mut SingleComponentVisitor {
                world: self.world,
                root,
                p: PhantomData::<T::First>,
            })?;
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
