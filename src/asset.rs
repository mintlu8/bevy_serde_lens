//! Module for serializing [`Handle`]s and [`Asset`]s.

use bevy::asset::uuid::Uuid;
use bevy::asset::{
    Asset, AssetId, AssetPath, AssetServer, Assets, Handle, UntypedAssetId, UntypedHandle,
};
use bevy_serde_lens_core::{DeUtils, SerUtils};
use ref_cast::RefCast;
use rustc_hash::FxHashMap;
use scoped_tls_hkt::scoped_thread_local;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;
use std::ops::Deref;

use crate::{MappedSerializer, MappedValue, derrorf, impl_with_notation_newtype, serrorf};

scoped_thread_local!(
    pub(crate) static mut SER_REUSABLE_HANDLES: FxHashMap<UntypedAssetId, usize>
);

scoped_thread_local!(
    pub(crate) static mut DE_REUSABLE_HANDLES: FxHashMap<usize, UntypedHandle>
);

#[derive(Serialize)]
#[serde(rename = "Id")]
pub(crate) enum SerHandleId<'t> {
    Path(&'t AssetPath<'static>),
    Index(usize),
    Uuid(Uuid),
}

#[derive(Serialize)]
#[serde(rename = "Asset", bound(serialize = ""))]
pub(crate) struct HandleSerialization<'t, T: Asset, M: MappedSerializer<T>> {
    pub index: SerHandleId<'t>,
    pub asset: Option<&'t MappedValue<T, M>>,
}

#[derive(Deserialize)]
#[serde(rename = "Id")]
pub(crate) enum DeHandleId {
    Path(AssetPath<'static>),
    Index(usize),
    //Uuid(Uuid),
}

#[derive(Deserialize)]
#[serde(rename = "Asset", bound(deserialize = ""))]
pub(crate) struct HandleDeserialization<T: Asset, M: MappedSerializer<T>> {
    pub index: DeHandleId,
    pub asset: Option<MappedValue<T, M>>,
}

/// Newtype of [`Handle`] that serializes its content.
///
/// # Rules
///
/// * Pathed: serialize its string path, deserialize load from that path. (If `PATHED` is true).
/// * Weak with uuid: assume its from `weak_from_u128` and serialize the uuid only.
/// * Strong: serialize an index and value on first occurrence.
///
/// # Errors
///
/// * Weak without uuid.
/// * Value missing.
#[derive(Debug, Clone, Default, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct SerializeHandle<T: Asset, M: MappedSerializer<T>, const PATHED: bool>(
    pub Handle<T>,
    PhantomData<M>,
);

impl<T: Asset, M: MappedSerializer<T>, const P: bool> SerializeHandle<T, M, P> {
    pub fn new(handle: Handle<T>) -> Self {
        Self(handle, PhantomData)
    }
}

/// Alias for [`SerializeHandle`] that supports serializing paths.
pub type PathedHandle<T> = SerializeHandle<T, (), true>;

/// Alias for [`SerializeHandle`] that forbids serializing paths.
pub type OwnedHandle<T> = SerializeHandle<T, (), false>;

/// Alias for [`SerializeHandle`] that supports serializing paths.
pub type MappedPathedHandle<T, M> = SerializeHandle<T, M, true>;

/// Alias for [`SerializeHandle`] that forbids serializing paths.
pub type MappedOwnedHandle<T, M> = SerializeHandle<T, M, false>;

impl<T: Asset, M: MappedSerializer<T>, const P: bool> Deref for SerializeHandle<T, M, P> {
    type Target = Handle<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Asset, M: MappedSerializer<T>, const P: bool> Serialize for SerializeHandle<T, M, P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerUtils::with_world::<S, _>(|world| {
            if P && let Some(path) = self.0.path() {
                return HandleSerialization::<T, M> {
                    index: SerHandleId::Path(path),
                    asset: None,
                }
                .serialize(serializer);
            }
            match self.0 {
                Handle::Strong(_) => {
                    let id = self.0.id().untyped();
                    SER_REUSABLE_HANDLES.with(|handles| {
                        let len = handles.len();
                        if let Some(prev) = handles.get(&id) {
                            return HandleSerialization::<T, M> {
                                index: SerHandleId::Index(*prev),
                                asset: None,
                            }
                            .serialize(serializer);
                        } else {
                            if let Some(assets) = world.get_resource::<Assets<T>>() {
                                if let Some(asset) = assets.get(self.0.id()) {
                                    handles.insert(id, len);
                                    return HandleSerialization::<T, M> {
                                        index: SerHandleId::Index(len),
                                        asset: Some(MappedValue::ref_cast(asset)),
                                    }
                                    .serialize(serializer);
                                }
                            }
                        }
                        Err(serrorf!(
                            "Handle {:?} does not have a corresponding asset.",
                            self.0
                        ))
                    })
                }
                Handle::Weak(asset_id) => match asset_id {
                    AssetId::Index { .. } => {
                        Err(serrorf!("Handle {:?} cannot be serialized.", self.0))
                    }
                    AssetId::Uuid { uuid } => HandleSerialization::<T, M> {
                        index: SerHandleId::Uuid(uuid),
                        asset: None,
                    }
                    .serialize(serializer),
                },
            }
        })?
    }
}

impl<'de, T: Asset, M: MappedSerializer<T>, const P: bool> Deserialize<'de>
    for SerializeHandle<T, M, P>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let handle = HandleDeserialization::<T, M>::deserialize(deserializer)?;
        DeUtils::with_world_mut::<D, _>(|world| {
            Ok(SerializeHandle::new(match handle.index {
                DeHandleId::Path(path) => {
                    let Some(asset_server) = world.get_resource::<AssetServer>() else {
                        return Err(derrorf!("AssetServer not found."));
                    };
                    asset_server.load(path)
                }
                DeHandleId::Index(id) => DE_REUSABLE_HANDLES.with(|handles| {
                    if let Some(value) = handle.asset {
                        let Some(mut assets) = world.get_resource_mut::<Assets<T>>() else {
                            return Err(derrorf!("AssetServer not found."));
                        };
                        let handle = assets.add(value.0);
                        handles.insert(id, handle.clone().untyped());
                        Ok(handle)
                    } else {
                        let Some(handle) = handles.get(&id) else {
                            return Err(derrorf!("Asset {} missing.", id));
                        };
                        Ok(handle.clone().typed())
                    }
                })?,
            }))
        })?
    }
}

impl_with_notation_newtype!(
    [T: Asset, M: MappedSerializer<T>, const P: bool] SerializeHandle [T, M, P]
    Handle<T>
);
