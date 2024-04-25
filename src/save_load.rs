use bevy_app::App;
use bevy_ecs::world::World;
use std::sync::Mutex;
use std::marker::PhantomData;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::DeserializeSeed;
use crate::typetagged::{TraitObject, DeserializeAnyFn, IntoTypeTagged, TypeTagServer, TYPETAG_SERVER};
use crate::{BatchSerialization, WORLD_MUT};

#[allow(unused)]
use crate::batch;

/// Extension methods on [`World`].
pub trait WorldExtension {
    /// Save a [`BatchSerialization`] type or a group created by [`batch!`].
    ///
    /// # What's a [`Serializer`]?
    ///
    /// Most `serde` frontends provide a serializer, like `serde_json::Serializer`.
    /// They typically wrap a [`std::io::Write`] and write to that stream.
    fn save<T: BatchSerialization, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error>;
    /// Load a [`BatchSerialization`] type.
    ///
    /// # What's a [`Deserializer`]?
    ///
    /// Most `serde` frontends provide a serializer, like `serde_json::Deserializer`.
    /// They typically wrap a [`std::io::Read`] and read from that stream.
    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error>;
    /// Create a [`Serialize`] type from a [`World`] and a [`BatchSerialization`] type.
    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S>;
    /// Create a [`DeserializeSeed`] type from a [`World`] and a [`BatchSerialization`] type.
    fn deserialize_lens<S: BatchSerialization>(&mut self) -> DeserializeLens<S>;
    /// Create a [`Deserialize`] scope from a [`World`].
    /// 
    /// [`ScopedDeserializeLens`] can be used inside the scope.
    fn scoped_deserialize_lens<T>(&mut self, f: impl FnOnce() -> T) -> T;
    /// Despawn all entities in a [`BatchSerialization`] type recursively.
    fn despawn_bound_objects<T: BatchSerialization>(&mut self);
    /// Register a type that can be deserialized dynamically.
    fn register_typetag<A: TraitObject, B: IntoTypeTagged<A>>(&mut self);
    /// Register a type that can be deserialized dynamically from a primitive.
    /// 
    /// Accepts a `Fn(T) -> Result<Out, String>` where T is `()`, `bool`, `i64`, `u64`, `f64`, `char`, `&str` or `&[u8]`.
    /// 
    /// # Example 
    /// ```
    /// // deserialize number as the default attacking type
    /// app.register_deserialize_any(|x: i64| Ok(DefaultAttack::new(x as i32)));
    /// ```
    fn register_deserialize_any<T: TraitObject, O>(&mut self, f: impl DeserializeAnyFn<T, O>);
}

impl WorldExtension for World {
    fn save<T: BatchSerialization, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error> {
        T::serialize(self, serializer)
    }

    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error> {
        self.init_resource::<TypeTagServer>();
        self.resource_scope::<TypeTagServer, _>(|world, server| {
            TYPETAG_SERVER.set(&server, || {
                WORLD_MUT.set(world, || T::De::deserialize(deserializer)).map(|_|())
            })
        })
    }

    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S> {
        SerializeLens(Mutex::new(self), PhantomData)
    }

    fn deserialize_lens<S: BatchSerialization>(&mut self) -> DeserializeLens<S> {
        DeserializeLens(self, PhantomData)
    }

    fn scoped_deserialize_lens<T>(&mut self, f: impl FnOnce() -> T) -> T {
        WORLD_MUT.set(self, f)
    }

    fn despawn_bound_objects<T: BatchSerialization>(&mut self){
        T::despawn(self)
    }

    fn register_typetag<A: TraitObject, B: IntoTypeTagged<A>>(&mut self){
        let mut server = self.get_resource_or_insert_with(TypeTagServer::default);
        server.register::<A, B>()
    }

    fn register_deserialize_any<T: TraitObject, O>(&mut self, f: impl DeserializeAnyFn<T, O>) {
        let mut server = self.get_resource_or_insert_with(TypeTagServer::default);
        server.register_deserialize_any::<T, O>(f)
    }
}

impl WorldExtension for App {
    fn save<T: BatchSerialization, S: Serializer>(&mut self, serializer: S) -> Result<S::Ok, S::Error> {
        self.world.save::<T, S>(serializer)
    }

    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(&mut self, deserializer: D) -> Result<(), D::Error> {
        self.world.load::<T, D>(deserializer)
    }

    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S> {
        self.world.serialize_lens()
    }

    fn deserialize_lens<S: BatchSerialization>(&mut self) -> DeserializeLens<S> {
        self.world.deserialize_lens()
    }

    fn scoped_deserialize_lens<T>(&mut self, f: impl FnOnce() -> T) -> T {
        self.world.scoped_deserialize_lens(f)
    }

    fn despawn_bound_objects<T: BatchSerialization>(&mut self){
        self.world.despawn_bound_objects::<T>()
    }

    fn register_typetag<A: TraitObject, B: IntoTypeTagged<A>>(&mut self){
        self.world.register_typetag::<A, B>()
    }

    fn register_deserialize_any<T: TraitObject, O>(&mut self, f: impl DeserializeAnyFn<T, O>) {
        self.world.register_deserialize_any::<T, O>(f)
    }
}


/// A [`Serialize`] type from a [`World`] reference and a [`BatchSerialization`] type.
pub struct SerializeLens<'t, S: BatchSerialization>(Mutex<&'t mut World>, PhantomData<S>);
 
impl<T: BatchSerialization> Serialize for SerializeLens<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.lock().unwrap().save::<T, S>(serializer)
    }
}

/// A [`DeserializeSeed`] type from a [`World`] reference and a [`BatchSerialization`] type.
pub struct DeserializeLens<'t, S: BatchSerialization>(&'t mut World, PhantomData<S>);

impl<'de, T: BatchSerialization> DeserializeSeed<'de> for DeserializeLens<'de, T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        self.0.load::<T, D>(deserializer)
    }
}

/// A [`Deserialize`] type from a [`BatchSerialization`] type.
/// 
/// Usable only in the `scoped_deserialize_lens` function's scope.
pub struct ScopedDeserializeLens<S: BatchSerialization>(PhantomData<S>);

impl<'de, T: BatchSerialization> Deserialize<'de> for ScopedDeserializeLens<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        T::De::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}