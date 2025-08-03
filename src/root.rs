use bevy::ecs::entity::Entity;
use bevy_serde_lens_core::{DeUtils, ScopeUtils};
use serde::{
    Deserialize, Deserializer,
    de::{SeqAccess, Visitor},
};
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::{BevyObject, ZstInit};

/// Building block item.
///
/// When deserialized in a `bevy_defer` scope, spawn a new entity with the item and return it.
#[derive(Debug, Clone, Copy)]
pub struct RootObject<T>(Entity, PhantomData<T>);

impl<T> RootObject<T> {
    pub fn get(&self) -> Entity {
        self.0
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for RootObject<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = DeUtils::with_world_mut::<D, _>(|w| w.spawn_empty().id())?;
        if let Err(e) =
            ScopeUtils::current_entity_scope(id, || T::Object::deserialize(deserializer))
        {
            DeUtils::with_world_mut::<D, _>(|w| {
                if let Ok(entity) = w.get_entity_mut(id) {
                    entity.despawn();
                }
            })?;
            return Err(e);
        }
        Ok(RootObject(id, PhantomData))
    }
}

/// Make a [`BevyObject`] [`Deserialize`] by providing root level entities in the world.
pub struct Root<T>(PhantomData<T>);

impl<T> Debug for Root<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Root").finish()
    }
}

impl<T> ZstInit for Root<T> {
    fn init() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T: BevyObject> Deserialize<'de> for Root<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(Root(PhantomData))
    }
}

impl<'de, T: BevyObject> Visitor<'de> for Root<T> {
    type Value = Root<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(item) = seq.next_element::<RootObject<T>>()? {
            DeUtils::with_world_mut_err::<A::Error, _>(|world| {
                if let Some(mut root) = T::get_root(world) {
                    root.add_child(item.get());
                }
            })?
        }
        Ok(Root(PhantomData))
    }
}
