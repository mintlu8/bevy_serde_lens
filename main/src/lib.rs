//! A pretty and structural validating serialization crate for the bevy engine.
//!
//! # Features
//!
//! * Serialize and deserialize structs with world access.
//! * Treat an [`Entity`], its [`Component`]s and children as a single serde object.
//! * Serialize [`Handle`]s and provide a generalized data interning interface.
//! * Deserialize trait objects like `Box<dyn T>`, as an alternative to `typetag`.
//!
//! # Getting Started
//!
//! Assume all components are [`Serialize`] and [`DeserializeOwned`].
//!
//! Serialize an [`Entity`] Character with some components and children:
//! ```
//! bind_object!(Character {
//!     #[serde(flatten)]
//!     character: Character,
//!     position: Position,
//!     #[serde(default, skip_serializing_if="Option::is_none")]
//!     weapon: Maybe<Weapon>,
//!     shield: Maybe<Shield>,
//!     #[serde(default, skip_serializing_if="Vec::is_empty")]
//!     potions: ChildVec<Potion>,
//! })
//! ```
//! 
//! Then call `save` on [`World`], where `serializer` is something like `serde_json::Serializer`.
//! ```
//! // Save
//! world.save::<Character>(serializer)
//! // Load
//! world.load::<Character>(deserializer)
//! // Delete
//! world.despawn_bound_objects::<Character>(deserializer)
//! ```
//! 
//! This saves a list of Characters like so:
//! ```
//! [
//!     { .. },
//!     { .. },
//!     ..
//! ]
//! ```
//!
//! To save multiple types of objects in a batch, create a batch serialization type with the [`batch!`] macro.
//!
//! ```
//! type SaveFileOne = batch!(Character, Monster, Terrain);
//! world.save::<SaveFileOne>(serializer)
//! world.load::<SaveFileOne>(serializer)
//! world.despawn_bound_objects::<SaveFileOne>(serializer)
//! ```
//!
//! This saves a map like so:
//! ```
//! {
//!     "Character": [ 
//!         { .. },
//!         { .. },
//!         ..
//!     ],
//!     "Monster": [ .. ],
//!     "Terrain": [ .. ]
//! }
//! ```
//! 
//! # The traits and what they do
//!
//! ## `Serialize` and `DeserializeOwned`
//! 
//! Any [`Serialize`] and [`DeserializeOwned`] type is automatically [`SerdeProject`] and 
//! any such [`Component`] is automatically a [`BevyObject`].
//! 
//! This comes with the downside that we cannot implement [`SerdeProject`] on any foreign
//! type due to the orphan rule. 
//! This is where [`Convert`] and the [`SerdeProject`](bevy_serde_project_derive::SerdeProject) 
//! macro comes in handy.
//!
//! ## `FromWorldAccess`
//!
//! A convenient trait for getting something from the world. 
//!
//! Either [`NoContext`],
//! a [`Resource`] or [`WorldAccess`] (`&world` and `&mut World`)
//!
//! ## `SerdeProject`
//!
//! [`SerdeProject`] projects non-serde types into serde types with world access.
//!
//! The [`SerdeProject`](bevy_serde_project_derive::SerdeProject) macro implements 
//! [`SerdeProject`] on type where all fields either implements [`SerdeProject`] or converts
//! to a [`SerdeProject`] newtype via the [`Convert`] trait.
//!
//! ### Example 
//!
//! Serialize a [`Handle`] as its path, stored in `AssetServer`.
//! 
//! ```
//! #[derive(SerdeProject)]
//! struct MySprite {
//!     // implements serde, therefore is `SerdeProject`.
//!     pub name: String,
//!     // Calls `Convert` and `PathHandle<Image>` is `SerdeProject`.
//!     #[serde_project("PathHandle<Image>")]
//!     pub handle: Handle<Image>
//! }
//! ```
//!
//! ## `BevyObject`
//!
//! A [`BevyObject`] allows an [`Entity`] to be serialized.
//! All [`SerdeProject`] [`Component`]s are `BevyObject`s
//! since each entity can only have at most one of each component.
//!
//! ## `BindBevyObject`
//!
//! [`BindBevyObject`] is a key [`Component`] that indicates an Entity is the [`BevyObject`].
//! Any entity that has the `Component` but does not satisfy the layout of the `BevyObject`
//! will result in an error.

use std::any::type_name;
use std::borrow::Borrow;
use std::fmt::Display;

use bevy_ecs::{component::Component, system::Resource, world::{EntityRef, EntityWorldMut}};
use ref_cast::RefCast;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
mod from_world;
pub use from_world::{NoContext, WorldAccess, FromWorldAccess, from_world, from_world_mut};
mod extractors;
pub use extractors::{Object, Maybe, Child, ChildUnchecked, ChildVec, ChildMap};
mod save_load;
pub use save_load::{WorldExtension, Join};
mod macros;
pub mod typetagged;
pub mod asset;
pub mod interning;
pub mod value;

pub use bevy_serde_project_derive::SerdeProject;

#[allow(unused)]
use bevy_asset::Handle;
#[allow(unused)]
use bevy_hierarchy::Children;

#[doc(hidden)]
pub use bevy_ecs::{world::World, entity::Entity};
use bevy_ecs::world::Mut;
#[doc(hidden)]
pub use serde;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("More than one children of {parent:?} found with extractor Child<{ty}>.")]
    MoreThenOne{
        parent: Entity,
        ty: &'static str,
    },
    #[error("Entity {0:?} missing.")]
    EntityMissing(Entity),
    #[error("Component {ty} not found for Entity {entity:?}.")]
    ComponentNotFound{
        entity: Entity,
        ty: &'static str,
    },
    #[error("ChildMap<{key}, {value}> found key with missing value.")]
    KeyNoValue{
        key: &'static str,
        value: &'static str,
    },
    #[error("Resource {ty} not found.")]
    ResourceNotFound{
        ty: &'static str,
    },
    #[error("Field {field} missing in BevyObject {ty}.")]
    FieldMissing{
        field: &'static str,
        ty: &'static str,
    },
    #[error("Unregistered type {name} of trait object {ty}.")]
    UnregisteredTraitObjectType {
        ty: &'static str,
        name: String,
    },
    #[error("Serialization Error: {0}")]
    SerializationError(String),
    #[error("Serialization Error: {0}")]
    DeserializationError(String),
    #[error("Cannot serialize a skipped enum variant \"{0}\".")]
    SkippedVariant(&'static str),
    #[error("{0}")]
    CustomError(String),
    #[error("Handle<{ty}> does not have an associated path.")]
    PathlessHandle{
        ty: &'static str
    },
    #[error("Associated Asset of Handle<{ty}> missing.")]
    AssetMissing{
        ty: &'static str
    },
    #[error("'__Phantom' branch deserialized, this is impossible.")]
    PhantomBranch,
    #[error("Trying to serialize/deserialize a enum with no valid variants.")]
    NoValidVariants,
}

impl Error {
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    pub fn custom(err: impl Display) -> Box<Self> {
        Box::new(Self::CustomError(err.to_string()))
    }
}

type BoxError = Box<Error>;

/// A type serializable and deserializable with [`World`] access.
pub trait SerdeProject: Sized {
    /// Context fetched from the [`World`].
    type Ctx: FromWorldAccess;

    /// A [`Serialize`] type.
    type Ser<'t>: Serialize + 't where Self: 't;
    /// A [`Deserialize`] type.
    type De<'de>: Deserialize<'de>;

    /// Convert to a [`Serialize`] type.
    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError>;
    /// Convert from a [`Deserialize`] type.
    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError>;
}

/// Alias for [`SerdeProject::Ser`].
pub type Ser<'t, T> = <T as SerdeProject>::Ser<'t>;
/// Alias for [`SerdeProject::De`].
pub type De<'de, T> = <T as SerdeProject>::De<'de>;

impl<T> SerdeProject for T where T: Serialize + DeserializeOwned + 'static {
    type Ctx = NoContext;
    type Ser<'t> = &'t Self;
    type De<'de> = Self;

    fn to_ser(&self, _: &()) -> Result<Self::Ser<'_>, BoxError> {
        Ok(self)
    }

    fn from_de(_: &mut (), de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(de)
    }
}

/// Associate a [`BevyObject`] to all [`Entity`]s with a specific [`Component`].
///
/// This means `world.save::<T>()` will try to serialize all entities with type T.
pub trait BindBevyObject: Component {
    type BevyObject: BevyObject;

    /// Obtain the root node to parent this component to if directly called.
    /// Default is `None`, which means no parent.
    #[allow(unused)]
    fn get_root(world: &mut World) -> Option<Entity> {
        None
    }

    /// Name of the object, must be unique.
    fn name() -> &'static str;
}

/// Treat an [`Entity`], its [`Component`]s and its [`Children`] as a serializable object.
///
/// All [`Serialize`] + [`DeserializeOwned`] components automatically implements this.
pub trait BevyObject {
    type Ser<'t>: Serialize + 't where Self: 't;
    type De<'de>: Deserialize<'de>;

    /// Convert to a [`Serialize`] type, returns [`None`] only if the entity is not found.
    #[allow(clippy::wrong_self_convention)]
    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError>;
    /// Convert from a [`Deserialize`] type.
    fn from_de(world: &mut World, entity: Entity, de: Self::De<'_>) -> Result<(), BoxError>;
}


impl<T> BevyObject for T where T: SerdeProject + Component {
    type Ser<'t> = T::Ser<'t> where T: 't;
    type De<'de> = T::De<'de>;

    #[allow(clippy::wrong_self_convention)]
    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let state = T::Ctx::from_world(world)?;
        match world.get_entity(entity).and_then(|e| e.get::<T>()) {
            Some(component) => component.to_ser(&state).map(Some),
            None => Ok(None),
        }
    }

    fn from_de(world: &mut World, entity: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let mut state = T::Ctx::from_world_mut(world)?;
        let result = T::from_de(&mut state, de)?;
        drop(state);
        world.entity_mut_ok(entity)?.insert(result);
        Ok(())
    }
}

/// Convert a foreign type to a [`SerdeProject`] type.
///
/// This is required for the `#[serde_project("MyNewType<..>")]` attribute.
pub trait Convert<In> {
    /// Convert a reference to a [`SerdeProject`] type's reference.
    ///
    /// A simple implementation is [`Clone`], but if
    /// using a newtype, you might want to use a crate like [`ref_cast`] to
    /// perform this conversion on a newtype.
    fn ser(input: &In) -> impl Borrow<Self>;
    /// Convert this [`SerdeProject`] type back to the original.
    fn de(self) -> In;
}

#[derive(Debug, RefCast)]
#[repr(transparent)]
/// A projection that serializes a [`Vec`] like container of [`SerdeProject`] types.
pub struct ProjectVec<T: FromIterator<A>, A: SerdeProject + 'static>(T) where for<'t> &'t T: IntoIterator<Item = &'t A>;

impl<T: FromIterator<A>, A: SerdeProject + 'static> Convert<T> for ProjectVec<T, A> where for<'t> &'t T: IntoIterator<Item = &'t A> {
    fn ser(input: &T) -> impl Borrow<Self> {
        ProjectVec::<T, A>::ref_cast(input)
    }

    fn de(self) -> T {
        self.0
    }
}

impl<T: FromIterator<A>, A: SerdeProject + 'static> SerdeProject for ProjectVec<T, A> where for<'t> &'t T: IntoIterator<Item = &'t A> {
    type Ctx = A::Ctx;

    type Ser<'t> = Vec<A::Ser<'t>> where T: 't;

    type De<'de> = Vec<A::De<'de>>;

    fn to_ser<'t>(&'t self, ctx: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        (&self.0).into_iter().map(|x| x.to_ser(ctx)).collect()
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(Self(de.into_iter().map(|de|A::from_de(ctx, de)).collect::<Result<_, _>>()?))
    }
}

/// [`World`] functions that return a [`Result`].
trait WorldUtil {
    fn entity_ok(&self, entity: Entity) -> Result<EntityRef, BoxError>;
    fn entity_mut_ok(&mut self, entity: Entity) -> Result<EntityWorldMut, BoxError>;
    fn resource_ok<R: Resource>(&self) -> Result<&R, BoxError>;
    fn resource_mut_ok<R: Resource>(&mut self) -> Result<Mut<'_, R>, BoxError>;
}

impl WorldUtil for World {
    fn entity_ok(&self, entity: Entity) -> Result<EntityRef, BoxError> {
        self.get_entity(entity)
            .ok_or_else(||Error::EntityMissing(entity).boxed())
    }

    fn entity_mut_ok(&mut self, entity: Entity) -> Result<EntityWorldMut, BoxError> {
        self.get_entity_mut(entity)
            .ok_or_else(||Error::EntityMissing(entity).boxed())
    }
    fn resource_ok<R: Resource>(&self) -> Result<&R, BoxError> {
        self.get_resource::<R>()
            .ok_or_else(||Error::ResourceNotFound { ty: type_name::<R>() }.boxed())
    }

    fn resource_mut_ok<R: Resource>(&mut self) -> Result<Mut<'_, R>, BoxError> {
        self.get_resource_mut::<R>()
            .ok_or_else(||Error::ResourceNotFound { ty: type_name::<R>() }.boxed())
    }
}