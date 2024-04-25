//! Module for serializing [`Handle`]s and [`Asset`]s.

use std::ops::Deref;
use std::path::PathBuf;
use bevy_asset::{Asset, AssetServer, Assets, Handle};
use ref_cast::RefCast;
use serde::{Deserialize, Serialize, Serializer};

use crate::{with_world, with_world_mut};

/// Projection of [`Handle`] that serializes its string path.
#[derive(Debug, Clone, Default, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct PathHandle<T: Asset>(pub Handle<T>);

impl<T: Asset> Deref for PathHandle<T> {
    type Target = Handle<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Asset> Serialize for PathHandle<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        with_world::<_, S>(|world| {
            let Some(asset_server) = world.get_resource::<AssetServer>() else {
                return Err(serde::ser::Error::custom("AssetServer not found."));
            };
            match asset_server.get_path(&self.0) {
                Some(path) => path.serialize(serializer),
                None => Err(serde::ser::Error::custom(
                    format!("Handle {:?} has no associated path.", self.0)
                )),
            }
        })?
    }
}

impl<'de, T: Asset> Deserialize<'de> for PathHandle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let path = PathBuf::deserialize(deserializer)?;
        with_world_mut::<_, D>(|world| {
            let Some(asset_server) = world.get_resource::<AssetServer>() else {
                return Err(serde::de::Error::custom("AssetServer not found."));
            };
            Ok(PathHandle(asset_server.load(path)))
        })?
    }
}

/// Projection of [`Handle`] that serializes its content.
#[derive(Debug, Clone, Default, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct UniqueHandle<T: Asset>(pub Handle<T>);

impl<T: Asset + Serialize> Serialize for UniqueHandle<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        with_world::<_, S>(|world| {
            let Some(assets) = world.get_resource::<Assets<T>>() else {
                return Err(serde::ser::Error::custom(
                    format!("Assets asset missing for handle {:?}.", self.0)
                ));
            };
            match assets.get(&self.0) {
                Some(asset) => asset.serialize(serializer),
                None => Err(serde::ser::Error::custom(
                    format!("Associated asset missing for handle {:?}.", self.0)
                )),
            }
        })?
    }
}

impl<'de, T: Asset + Deserialize<'de>> Deserialize<'de> for UniqueHandle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let path = T::deserialize(deserializer)?;
        with_world_mut::<_, D>(|world| {
            let Some(mut assets) = world.get_resource_mut::<Assets<T>>() else {
                return Err(serde::de::Error::custom("AssetServer not found."));
            };
            Ok(UniqueHandle(assets.add(path)))
        })?
    }
}

/// to be used `#[serde(with = path_handle)]`
pub mod path_handle {

}

/// to be used `#[serde(with = path_handle)]`
pub mod unique_handle {

}