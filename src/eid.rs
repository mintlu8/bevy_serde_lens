use std::cell::RefCell;

use bevy_ecs::entity::Entity;
use bevy_hierarchy::{BuildWorldChildren, Parent};
use ref_cast::RefCast;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    world_entity_scope, world_entity_scope_mut, BindProject, BindProjectQuery, Maybe, ZstInit,
    ENTITY,
};

thread_local! {
    pub static EID_MAP: RefCell<FxHashMap<u64, Entity>> = RefCell::new(FxHashMap::default());
}

/// Serialize [`Entity`] as a number for future reference.
///
/// When used with `#[serde(with = "EntityId")]`:
///
/// * On serialization: Save a unique id for this entity.
///
/// * On deserialization: Associate the unique id to its [`Entity`] for future use.
pub struct EntityId;

impl EntityId {
    /// Serialize with [`EntityId`].
    pub fn serialize<S: serde::Serializer>(
        item: &Entity,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        item.to_bits().serialize(serializer)
    }

    /// Deserialize with [`EntityId`].
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Entity, D::Error> {
        let original = u64::deserialize(deserializer)?;
        EID_MAP
            .with(|e| e.borrow().get(&original).copied())
            .ok_or(serde::de::Error::custom("EntityId not serialized."))
    }
}

impl Serialize for EntityId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entity = ENTITY.with(|e| *e);
        entity.to_bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entity = u64::deserialize(deserializer)?;
        let current = ENTITY.with(|e| *e);
        EID_MAP.with(|x| x.borrow_mut().insert(entity, current));
        Ok(EntityId)
    }
}

impl ZstInit for EntityId {
    fn init() -> Self {
        EntityId
    }
}

impl BindProject for EntityId {
    type To = Self;
}

impl BindProjectQuery for EntityId {
    type Data = Entity;
}

/// Parent this entity to an entity via its previously serialized [`EntityId`].
///
/// Due to implementation reasons, this is currently not validated on the serialization step.
pub struct Parented;

impl Serialize for Parented {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Some(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            let Some(component) = entity.get::<Parent>() else {
                return Err(serde::ser::Error::custom("Parent missing."));
            };
            component.get().serialize(serializer)
        })?
    }
}

impl<'de> Deserialize<'de> for Parented {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let original = u64::deserialize(deserializer)?;
        let Some(parent) = EID_MAP.with(|e| e.borrow().get(&original).copied()) else {
            return Err(serde::de::Error::custom("Parent not serialized."));
        };
        world_entity_scope_mut::<_, D>(|world, entity| {
            let Some(mut entity) = world.get_entity_mut(entity) else {
                return Err(serde::de::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            entity.set_parent(parent);
            Ok(Parented)
        })?
    }
}

impl ZstInit for Parented {
    fn init() -> Self {
        Parented
    }
}

impl BindProject for Parented {
    type To = Self;
}

impl BindProjectQuery for Parented {
    type Data = &'static Parent;
}

impl Serialize for Maybe<Parented> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Some(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            entity
                .get::<Parent>()
                .map(|x| x.get())
                .serialize(serializer)
        })?
    }
}

impl<'de> Deserialize<'de> for Maybe<Parented> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let original = <Option<u64>>::deserialize(deserializer)?;
        if let Some(original) = original {
            let Some(parent) = EID_MAP.with(|e| e.borrow().get(&original).copied()) else {
                return Err(serde::de::Error::custom("Parent not serialized."));
            };
            world_entity_scope_mut::<_, D>(|world, entity| {
                if let Some(mut entity) = world.get_entity_mut(entity) {
                    entity.set_parent(parent);
                    Ok(())
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Entity missing: {entity:?}."
                    )))
                }
            })??;
        }
        Ok(Maybe::init())
    }
}

impl BindProject for Maybe<Parented> {
    type To = Self;
}

impl BindProjectQuery for Maybe<Parented> {
    type Data = Option<&'static Parent>;
}

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct EntityPtr(pub Entity);

impl Serialize for EntityPtr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityPtr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let original = <u64>::deserialize(deserializer)?;
        let Some(entity) = EID_MAP.with(|e| e.borrow().get(&original).copied()) else {
            return Err(serde::de::Error::custom("Parent not serialized."));
        };
        Ok(EntityPtr(entity))
    }
}

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct OptionEntityPtr(pub Option<Entity>);

impl Serialize for OptionEntityPtr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.map(|x| x.to_bits()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OptionEntityPtr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entity = <Option<u64>>::deserialize(deserializer)?
            .and_then(|entity| EID_MAP.with(|e| e.borrow().get(&entity).copied()));
        Ok(OptionEntityPtr(entity))
    }
}
