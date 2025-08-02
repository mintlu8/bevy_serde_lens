use crate::typetagged::TYPETAG_SERVER;
use crate::typetagged::{ErasedObject, TypeTagServer};
use crate::BatchSerialization;
use bevy::app::App;
use bevy::ecs::resource::Resource;
use bevy::ecs::world::World;
use bevy::reflect::TypePath;
use bevy_serde_lens_core::ScopeUtils;
use serde::de::DeserializeOwned;
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
    fn save<T: BatchSerialization, S: Serializer>(
        &mut self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>;
    /// Load a [`BatchSerialization`] type.
    ///
    /// # What's a [`Deserializer`]?
    ///
    /// Most `serde` frontends provide a serializer, like `serde_json::Deserializer`.
    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(
        &mut self,
        deserializer: D,
    ) -> Result<(), D::Error>;
    /// Create a [`Serialize`] type from a [`World`] and a [`BatchSerialization`] type.
    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S>;
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

    /// When serializing, extract a resource into a thread local scope.
    ///
    /// To implement this, push `R` into a scope then call the `FnMut`,
    /// it is recommended to use [`scoped_tls_hkt`] alongside this.
    fn register_serialize_resource_cx<R: Resource>(
        &mut self,
        extract: impl Fn(&R, &mut dyn FnMut()) + Send + Sync + 'static,
    );

    /// When deserializing, extract a resource into a thread local scope.
    ///
    /// To implement this, push `R` into a scope then call the `FnMut`,
    /// it is recommended to use [`scoped_tls_hkt`] alongside this.
    fn register_deserialize_resource_cx<R: Resource>(
        &mut self,
        extract: impl Fn(&mut R, &mut dyn FnMut()) + Send + Sync + 'static,
    );
}

impl WorldExtension for World {
    fn save<T: BatchSerialization, S: Serializer>(
        &mut self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        self.init_resource::<RegisteredExtractions>();
        let mut serializer = Some(serializer);
        let mut result = None;

        self.resource_scope::<RegisteredExtractions, _>(|world, extractions| {
            (extractions.ser)(world, &mut |world| {
                result = Some(T::serialize(world, serializer.take().unwrap()))
            })
        });
        result.unwrap()
    }

    fn load<'de, T: BatchSerialization, D: Deserializer<'de>>(
        &mut self,
        deserializer: D,
    ) -> Result<(), D::Error> {
        self.init_resource::<RegisteredExtractions>();
        let mut deserializer = Some(deserializer);
        let mut result = None;

        self.resource_scope::<RegisteredExtractions, _>(|world, extractions| {
            (extractions.de)(world, &mut |world| {
                result = Some(ScopeUtils::deserialize_scope(world, || {
                    T::De::deserialize(deserializer.take().unwrap())
                }))
            })
        });
        // Discard the zst.
        result.unwrap().map(|_| ())
    }

    fn serialize_lens<S: BatchSerialization>(&mut self) -> SerializeLens<S> {
        SerializeLens(Mutex::new(self), PhantomData)
    }

    fn deserialize_scope<T>(&mut self, f: impl FnOnce() -> T) -> T {
        self.init_resource::<RegisteredExtractions>();
        let mut f = Some(f);
        let mut result = None;
        self.resource_scope::<RegisteredExtractions, _>(|world, extractions| {
            (extractions.de)(world, &mut |world| {
                result = Some(ScopeUtils::deserialize_scope(world, f.take().unwrap()))
            })
        });
        result.unwrap()
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

    fn register_serialize_resource_cx<R: Resource>(
        &mut self,
        extract: impl Fn(&R, &mut dyn FnMut()) + Send + Sync + 'static,
    ) {
        self.init_resource::<RegisteredExtractions>();
        let mut res = self.resource_mut::<RegisteredExtractions>();
        res.ser = Box::new(move |world, callback| {
            if world.contains_resource::<R>() {
                world.resource_scope::<R, _>(|world, res| extract(&res, &mut || callback(world)))
            } else {
                callback(world)
            }
        })
    }

    fn register_deserialize_resource_cx<R: Resource>(
        &mut self,
        extract: impl Fn(&mut R, &mut dyn FnMut()) + Send + Sync + 'static,
    ) {
        self.init_resource::<RegisteredExtractions>();
        let mut res = self.resource_mut::<RegisteredExtractions>();
        res.de = Box::new(move |world, callback| {
            if world.contains_resource::<R>() {
                world.resource_scope::<R, _>(|world, mut res| {
                    extract(&mut res, &mut || callback(world))
                })
            } else {
                callback(world)
            }
        })
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

    fn register_serialize_resource_cx<R: Resource>(
        &mut self,
        extract: impl Fn(&R, &mut dyn FnMut()) + Send + Sync + 'static,
    ) {
        self.world_mut().register_serialize_resource_cx(extract);
    }

    fn register_deserialize_resource_cx<R: Resource>(
        &mut self,
        extract: impl Fn(&mut R, &mut dyn FnMut()) + Send + Sync + 'static,
    ) {
        self.world_mut().register_deserialize_resource_cx(extract);
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

/// A [`Deserialize`] type from a [`BatchSerialization`] type.
///
/// Usable only in the `deserialize_scope` function's scope.
pub struct InWorld<S: BatchSerialization>(PhantomData<S>);

impl<'de, T: BatchSerialization> Deserialize<'de> for InWorld<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::De::deserialize(deserializer)?;
        Ok(Self(PhantomData))
    }
}

#[derive(Resource)]
pub struct RegisteredExtractions {
    ser: Box<dyn Fn(&mut World, &mut dyn FnMut(&mut World)) + Send + Sync>,
    de: Box<dyn Fn(&mut World, &mut dyn FnMut(&mut World)) + Send + Sync>,
}

impl Default for RegisteredExtractions {
    fn default() -> Self {
        Self {
            ser: Box::new(|world, callback| callback(world)),
            de: Box::new(|world, callback| {
                if world.contains_resource::<TypeTagServer>() {
                    world.resource_scope::<TypeTagServer, _>(|world, server| {
                        TYPETAG_SERVER.set(&server, || callback(world))
                    })
                } else {
                    callback(world)
                }
            }),
        }
    }
}
