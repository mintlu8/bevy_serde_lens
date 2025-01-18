use crate::entity::EID_MAP;
use crate::typetagged::TYPETAG_SERVER;
use crate::typetagged::{ErasedObject, TypeTagServer};
use crate::{de_scope, BatchSerialization};
use bevy_app::App;
use bevy_ecs::world::World;
use bevy_reflect::TypePath;
use serde::de::{DeserializeOwned, DeserializeSeed};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;
use std::sync::Mutex;

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
    fn save<T: BatchSerialization, S: Serializer>(
        &mut self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>;
    /// Load a [`BatchSerialization`] type.
    ///
    /// # What's a [`Deserializer`]?
    ///
    /// Most `serde` frontends provide a serializer, like `serde_json::Deserializer`.
    /// They typically wrap a [`std::io::Read`] and read from that stream.
    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(
        &mut self,
        deserializer: D,
    ) -> Result<(), D::Error>;
    /// Create a [`Serialize`] type from a [`World`] and a [`BatchSerialization`] type.
    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S>;
    /// Create a [`DeserializeSeed`] type from a [`World`] and a [`BatchSerialization`] type.
    fn deserialize_lens<S: BatchSerialization>(&mut self) -> DeserializeLens<S>;
    /// Create a [`Deserialize`] scope from a [`World`].
    ///
    /// [`InWorld`] can be used inside the scope.
    fn deserialize_scope<T>(&mut self, f: impl FnOnce() -> T) -> T;
    /// Despawn all entities in a [`BatchSerialization`] type recursively.
    fn despawn_bound_objects<T: BatchSerialization>(&mut self);
    /// Register a type that can be deserialized via a type tag.
    ///
    /// The name of the type is [`TypePath::short_type_path`] and must be unique.
    fn register_typetag<A: ErasedObject, B: Into<A> + TypePath + DeserializeOwned>(&mut self);
    /// Register a type that can be deserialized via a type tag.
    ///
    /// The name of the type is specified by the caller.
    fn register_typetag_by_name<A: ErasedObject, B: Into<A> + DeserializeOwned>(
        &mut self,
        name: &str,
    );
}

impl WorldExtension for World {
    fn save<T: BatchSerialization, S: Serializer>(
        &mut self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        T::serialize(self, serializer)
    }

    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(
        &mut self,
        deserializer: D,
    ) -> Result<(), D::Error> {
        EID_MAP.with(|m| m.borrow_mut().clear());
        self.init_resource::<TypeTagServer>();
        self.resource_scope::<TypeTagServer, _>(|world, server| {
            TYPETAG_SERVER.set(&server, || {
                de_scope(world, || T::De::deserialize(deserializer)).map(|_| ())
            })
        })
    }

    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S> {
        SerializeLens(Mutex::new(self), PhantomData)
    }

    fn deserialize_lens<S: BatchSerialization>(&mut self) -> DeserializeLens<S> {
        DeserializeLens(self, PhantomData)
    }

    fn deserialize_scope<T>(&mut self, f: impl FnOnce() -> T) -> T {
        de_scope(self, f)
    }

    fn despawn_bound_objects<T: BatchSerialization>(&mut self) {
        T::despawn(self);
        // needed because of hooks.
        self.flush();
    }

    fn register_typetag<A: ErasedObject, B: Into<A> + TypePath + DeserializeOwned>(&mut self) {
        let mut server = self.get_resource_or_insert_with(TypeTagServer::default);
        server.register::<A, B>()
    }

    fn register_typetag_by_name<A: ErasedObject, B: Into<A> + DeserializeOwned>(
        &mut self,
        name: &str,
    ) {
        let mut server = self.get_resource_or_insert_with(TypeTagServer::default);
        server.register_by_name::<A, B>(name)
    }
}

impl WorldExtension for App {
    fn save<T: BatchSerialization, S: Serializer>(
        &mut self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        self.world_mut().save::<T, S>(serializer)
    }

    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(
        &mut self,
        deserializer: D,
    ) -> Result<(), D::Error> {
        self.world_mut().load::<T, D>(deserializer)
    }

    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S> {
        self.world_mut().serialize_lens()
    }

    fn deserialize_lens<S: BatchSerialization>(&mut self) -> DeserializeLens<S> {
        self.world_mut().deserialize_lens()
    }

    fn deserialize_scope<T>(&mut self, f: impl FnOnce() -> T) -> T {
        self.world_mut().deserialize_scope(f)
    }

    fn despawn_bound_objects<T: BatchSerialization>(&mut self) {
        self.world_mut().despawn_bound_objects::<T>()
    }

    fn register_typetag<A: ErasedObject, B: Into<A> + TypePath + DeserializeOwned>(&mut self) {
        self.world_mut().register_typetag::<A, B>()
    }

    fn register_typetag_by_name<A: ErasedObject, B: Into<A> + DeserializeOwned>(
        &mut self,
        name: &str,
    ) {
        self.world_mut().register_typetag_by_name::<A, B>(name)
    }
}

/// A [`Serialize`] type from a [`World`] reference and a [`BatchSerialization`] type.
pub struct SerializeLens<'t, S: BatchSerialization>(Mutex<&'t mut World>, PhantomData<S>);

impl<T: BatchSerialization> Serialize for SerializeLens<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.lock().unwrap().save::<T, S>(serializer)
    }
}

/// A [`DeserializeSeed`] type from a [`World`] reference and a [`BatchSerialization`] type.
pub struct DeserializeLens<'t, S: BatchSerialization>(&'t mut World, PhantomData<S>);

impl<'de, T: BatchSerialization> DeserializeSeed<'de> for DeserializeLens<'de, T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        EID_MAP.with(|m| m.borrow_mut().clear());
        self.0.load::<T, D>(deserializer)
    }
}

/// A [`Deserialize`] type from a [`BatchSerialization`] type.
///
/// Usable only in the `deserialize_scope` function's scope.
pub struct InWorld<S: BatchSerialization>(PhantomData<S>);

impl<'de, T: BatchSerialization> Deserialize<'de> for InWorld<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        EID_MAP.with(|m| m.borrow_mut().clear());
        T::De::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}
