use std::cell::RefCell;

use bevy_ecs::entity::Entity;
use bevy_hierarchy::{BuildWorldChildren, Parent};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{world_entity_scope, world_entity_scope_mut, BindProject, BindProjectQuery, Maybe, ZstInit, ENTITY};

thread_local! {
    pub static EID_MAP: RefCell<FxHashMap<Entity, Entity>> = RefCell::new(FxHashMap::default());
}

/// Serialize [`Entity`] as a number for future reference.
/// 
/// Doubles as a serialization method for [`Entity`] with `#[serde(with = "EntityId")]`.
pub struct EntityId;

impl EntityId {
    /// Serialize with [`EntityId`].
    pub fn serialize<S: serde::Serializer>(item: &Entity, serializer: S) -> Result<S::Ok, S::Error> {
        item.serialize(serializer)
    }

    /// Deserialize with [`EntityId`].
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Entity, D::Error> {
        let original = Entity::deserialize(deserializer)?;
        EID_MAP.with(|e| e.borrow().get(&original).copied()).ok_or(
            serde::de::Error::custom(
                "EntityId not serialized."
            )
        )
    }
}

impl Serialize for EntityId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entity = ENTITY.with(|e| *e);
        entity.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entity = Entity::deserialize(deserializer)?;
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

/// Parent this entity to an entity via previously serialized [`EntityId`].
pub struct Parented;

impl Serialize for Parented {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return Err(serde::ser::Error::custom(format!(
                        "Entity missing: {entity:?}."
                    )));
                };
                let Some(component) = entity.get::<Parent>() else {
                    return Err(serde::ser::Error::custom(
                        "Parent missing."
                    ));
                };
                component.get().serialize(serializer)
            }
        )?
    }
}

impl<'de> Deserialize<'de> for Parented {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let original = Entity::deserialize(deserializer)?;
        let Some(parent) = EID_MAP.with(|e| e.borrow().get(&original).copied()) else {
            return Err(serde::de::Error::custom(
                "Parent not serialized."
            ));
        };
        world_entity_scope_mut::<_, D>(
            |world, entity| {
                let Some(mut entity) = world.get_entity_mut(entity) else {
                    return Err(serde::de::Error::custom(format!(
                        "Entity missing {entity:?}."
                    )));
                };
                entity.set_parent(parent);
                Ok(Parented)
            }
        )?
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
        world_entity_scope::<_, S>(
            |world, entity| {
                let Some(entity) = world.get_entity(entity) else {
                    return Err(serde::ser::Error::custom(format!(
                        "Entity missing: {entity:?}."
                    )));
                };
                entity.get::<Parent>().map(|x| x.get()).serialize(serializer)
            }
        )?
    }
}

impl<'de> Deserialize<'de> for Maybe<Parented> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let original = <Option<Entity>>::deserialize(deserializer)?;
        if let Some(original) = original {
            let Some(parent) = EID_MAP.with(|e| e.borrow().get(&original).copied()) else {
                return Err(serde::de::Error::custom(
                    "Parent not serialized."
                ));
            };
            world_entity_scope_mut::<_, D>(
                |world, entity| {
                    if let Some(mut entity) = world.get_entity_mut(entity) {
                        entity.set_parent(parent);
                        Ok(())
                    } else{
                        Err(serde::de::Error::custom(format!(
                            "Entity missing: {entity:?}."
                        )))
                    }
                }
            )??;
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
