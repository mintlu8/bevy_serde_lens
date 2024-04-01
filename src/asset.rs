//! Module for serializing [`Handle`]s and [`Asset`]s.

use std::any::type_name;
use std::path::PathBuf;
use bevy_asset::{Asset, AssetServer, Assets, Handle};
use bevy_ecs::world::World;
use ref_cast::RefCast;
use serde::{Deserialize, Serialize};
use crate::{BoxError, Convert, Error, FromWorldAccess, SerdeProject, WorldAccess, WorldUtil};

/// Projection of [`Handle`] that serializes its string path.
#[derive(Debug, Clone, Default, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct PathHandle<T: Asset>(pub Handle<T>);

impl<T: Asset> Convert<Handle<T>> for PathHandle<T>{
    fn ser(input: &Handle<T>) -> &Self {
        Self::ref_cast(input)
    }

    fn de(self) -> Handle<T> {
        self.0
    }
}

impl<T: Asset> SerdeProject for PathHandle<T>{
    type Ctx = AssetServer;
    type Ser<'t> = PathBuf;
    type De<'de> = PathBuf;

    fn to_ser<'t>(&'t self, asset_server: &<Self::Ctx as FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, BoxError> {
        match asset_server.get_path(&self.0) {
            Some(path) => Ok(path.path().to_owned()),
            None => Err(Error::PathlessHandle { ty: type_name::<T>() }.boxed()),
        }
    }

    fn from_de(asset_server: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, BoxError> {
        Ok(PathHandle(asset_server.load(de)))
    }
}


/// Projection of [`Handle`] that serializes its content.
#[derive(Debug, Clone, Default, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct UniqueHandle<T: Asset + SerdeProject>(pub Handle<T>);

impl<T: Asset + SerdeProject> Convert<Handle<T>> for UniqueHandle<T>{
    fn ser(input: &Handle<T>) -> &Self {
        Self::ref_cast(input)
    }

    fn de(self) -> Handle<T> {
        self.0
    }
}

impl<T: Asset + SerdeProject> SerdeProject for UniqueHandle<T>{
    type Ctx = WorldAccess;
    type Ser<'t> = T::Ser<'t>;
    type De<'de> = T::De<'de>;

    fn to_ser<'t>(&'t self, world: &&'t World) -> Result<Self::Ser<'t>, BoxError> {
        match world.resource_ok::<Assets<T>>()?.get(&self.0) {
            Some(asset) => asset.to_ser(&<T::Ctx as FromWorldAccess>::from_world(world)?),
            None => Err(Error::AssetMissing { ty: type_name::<T>() }.boxed()),
        }
    }

    fn from_de(world: &mut &mut World, de: Self::De<'_>) -> Result<Self, BoxError> {
        let item = T::from_de(&mut <T::Ctx as FromWorldAccess>::from_world_mut(world)?, de)?;
        Ok(UniqueHandle(world.resource_mut_ok::<Assets<T>>()?.add(item)))
    }
}

/// Custom serialization for an [`Asset`].
pub trait SerdeAsset: Asset + Sized {
    /// A [`Serialize`] type.
    type Ser<'t>: Serialize + 't where Self: 't;
    /// A [`Deserialize`] type.
    type De<'de>: Deserialize<'de>;

    /// Convert to a [`Serialize`] type.
    fn to_ser<'t>(this: &'t Handle<Self>, ctx: &'t World) -> Result<Self::Ser<'t>, Box<Error>>;
    /// Convert from a [`Deserialize`] type.
    fn from_de(ctx: &mut World, de: Self::De<'_>) -> Result<Handle<Self>, Box<Error>>;
}

/// Projection of [`Handle`] that serializes
/// by the [`Asset`]'s [`SerdeAsset`] implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct SerdeHandle<T: SerdeAsset>(pub Handle<T>);

impl<T: SerdeAsset> Convert<Handle<T>> for SerdeHandle<T>{
    fn ser(input: &Handle<T>) -> &Self {
        Self::ref_cast(input)
    }

    fn de(self) -> Handle<T> {
        self.0
    }
}

impl<T: SerdeAsset> SerdeProject for SerdeHandle<T> {
    type Ctx = WorldAccess;
    type Ser<'t> = T::Ser<'t>;
    type De<'de> = T::De<'de>;

    fn to_ser<'t>(&'t self, world: &&'t World) -> Result<Self::Ser<'t>, BoxError> {
        SerdeAsset::to_ser(&self.0, world)
    }

    fn from_de(world: &mut &mut World, de: Self::De<'_>) -> Result<Self, BoxError> {
        SerdeAsset::from_de(world, de).map(SerdeHandle)
    }
}