//! Module for serializing [`Entity`] and hierarchy.
use std::cell::RefCell;

use bevy_ecs::{entity::Entity, query::With};
use bevy_hierarchy::{BuildWorldChildren, Parent};
use bevy_serde_lens_core::current_entity;
use ref_cast::RefCast;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    world_entity_scope, world_entity_scope_mut, BindProject, BindProjectQuery, Maybe, ZstInit,
};

thread_local! {
    pub(crate) static EID_MAP: RefCell<FxHashMap<u64, Entity>> = RefCell::new(FxHashMap::default());
}

#[cfg(any(feature = "extra-checks", debug_assertions))]
macro_rules! validate {
    ($val: expr) => {{
        let val = $val;
        if EID_MAP.with(|x| x.borrow().contains_key(&val)) {
            return Err(serde::ser::Error::custom("EID not serialized"));
        }
        val
    }};
    (@$val: expr) => {{
        let val = $val;
        if let Some(val) = val {
            if EID_MAP.with(|x| x.borrow().contains_key(&val)) {
                return Err(serde::ser::Error::custom("EID not serialized"));
            }
        }
        val
    }};
}

#[cfg(not(any(feature = "extra-checks", debug_assertions)))]
macro_rules! validate {
    ($val: expr) => {
        $val
    };
    (@$val: expr) => {
        $val
    };
}

/// Obtain the [`Entity`] of a serialized [`EntityId`].
///
/// # Errors
///
/// * If used outside a deserialize implementation.
/// * If used outside `bevy_serde_lens`.
/// * If [`EntityId`] is not serialized in the same batch.
pub fn validate<'de, S: Deserializer<'de>>(id: u64) -> Result<Entity, S::Error> {
    EID_MAP
        .with(|x| x.borrow().get(&id).copied())
        .ok_or_else(|| serde::de::Error::custom(format!("Entity {id} not serialized.")))
}

/// Obtain the [`Entity`] of a serialized [`EntityId`].
///
/// # Errors
///
/// * If used outside a deserialize implementation.
/// * If used outside `bevy_serde_lens`.
/// * If [`EntityId`] is not serialized in the same batch.
pub fn get_entity<'de, S: Deserializer<'de>>(id: u64) -> Result<Entity, S::Error> {
    EID_MAP
        .with(|x| x.borrow().get(&id).copied())
        .ok_or_else(|| serde::de::Error::custom(format!("Entity {id} not serialized.")))
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
    pub fn serialize<S: Serializer>(item: &Entity, serializer: S) -> Result<S::Ok, S::Error> {
        item.to_bits().serialize(serializer)
    }

    /// Deserialize with [`EntityId`].
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Entity, D::Error> {
        let original = u64::deserialize(deserializer)?;
        EID_MAP
            .with(|e| e.borrow().get(&original).copied())
            .ok_or(serde::de::Error::custom("EntityId not serialized."))
    }
}

impl Serialize for EntityId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entity = current_entity().map_err(serde::ser::Error::custom)?;
        entity.to_bits().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entity = u64::deserialize(deserializer)?;
        let current = current_entity().map_err(serde::de::Error::custom)?;
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
    type Filter = ();
}

impl BindProjectQuery for EntityId {
    type Data = query::SerializeEntity;
}

/// Parent this entity to an entity via its previously serialized [`EntityId`].
///
/// # Errors
///
/// If associated [`EntityId`] was not serialized.
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
            validate!(component.get().to_bits()).serialize(serializer)
        })?
    }
}

impl<'de> Deserialize<'de> for Parented {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
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
    type Filter = With<Parent>;
}

impl BindProjectQuery for Parented {
    type Data = query::SerializeParent;
}

impl Serialize for Maybe<Parented> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        world_entity_scope::<_, S>(|world, entity| {
            let Some(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            validate!(@entity
                .get::<Parent>()
                .map(|x| x.get().to_bits()))
            .serialize(serializer)
        })?
    }
}

impl<'de> Deserialize<'de> for Maybe<Parented> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let original = <Option<u64>>::deserialize(deserializer)?;
        if let Some(original) = original {
            let parent = get_entity::<D>(original)?;
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
    type Filter = ();
}

impl BindProjectQuery for Maybe<Parented> {
    type Data = query::SerializeMaybeParent;
}

/// Projection type of an [`Entity`].
///
/// When used with `#[serde(with = "EntityPtr")]`:
///
/// * On serialization: Save a unique id for this [`Entity`].
/// * On deserialization: Find the [`Entity`] of a previously serialized [`EntityId`].
///
/// # Errors
///
/// If associated [`EntityId`] was not serialized.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct EntityPtr(pub Entity);

impl Serialize for EntityPtr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        validate!(self.0.to_bits()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityPtr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let original = <u64>::deserialize(deserializer)?;
        Ok(EntityPtr(get_entity::<D>(original)?))
    }
}

/// Projection type of an [`Option<Entity>`].
///
/// When used with `#[serde(with = "OptionEntityPtr")]`:
///
/// * On serialization: Save a unique id for this [`Entity`].
/// * On deserialization: Find the [`Entity`] of a previously serialized [`EntityId`].
///
/// # Errors
///
/// If associated [`EntityId`] was not serialized.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct OptionEntityPtr(pub Option<Entity>);

impl Serialize for OptionEntityPtr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        validate!(@self.0.map(|x| x.to_bits())).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OptionEntityPtr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entity = <Option<u64>>::deserialize(deserializer)?;
        match entity {
            Some(id) => Ok(OptionEntityPtr(Some(get_entity::<D>(id)?))),
            None => Ok(OptionEntityPtr(None)),
        }
    }
}

impl EntityPtr {
    /// Serialize with [`EntityPtr`].
    pub fn serialize<S: Serializer>(item: &Entity, serializer: S) -> Result<S::Ok, S::Error> {
        Serialize::serialize(EntityPtr::ref_cast(item), serializer)
    }

    /// Deserialize with [`EntityPtr`].
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Entity, D::Error> {
        <EntityPtr as Deserialize>::deserialize(deserializer).map(|x| x.0)
    }
}

impl OptionEntityPtr {
    /// Serialize with [`OptionEntityPtr`].
    pub fn serialize<S: Serializer>(
        item: &Option<Entity>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Serialize::serialize(OptionEntityPtr::ref_cast(item), serializer)
    }

    /// Deserialize with [`OptionEntityPtr`].
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Entity>, D::Error> {
        <OptionEntityPtr as Deserialize>::deserialize(deserializer).map(|x| x.0)
    }
}

pub(crate) mod query {
    #[allow(unused)]
    use super::EID_MAP;
    use bevy_ecs::{entity::Entity, query::QueryData};
    use bevy_hierarchy::Parent;
    use serde::{Serialize, Serializer};

    #[derive(Debug, QueryData)]
    pub struct SerializeEntity {
        entity: Entity,
    }

    impl Serialize for SerializeEntity {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            validate!(self.entity.to_bits()).serialize(serializer)
        }
    }

    impl Serialize for SerializeEntityItem<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            validate!(self.entity.to_bits()).serialize(serializer)
        }
    }

    #[derive(Debug, QueryData)]
    pub struct SerializeParent {
        parent: &'static Parent,
    }

    impl Serialize for SerializeParent {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            validate!(self.parent.to_bits()).serialize(serializer)
        }
    }

    impl Serialize for SerializeParentItem<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            validate!(self.parent.to_bits()).serialize(serializer)
        }
    }

    #[derive(Debug, QueryData)]
    pub struct SerializeMaybeParent {
        parent: Option<&'static Parent>,
    }

    impl Serialize for SerializeMaybeParent {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            validate!(@self.parent.map(|x| x.to_bits())).serialize(serializer)
        }
    }

    impl Serialize for SerializeMaybeParentItem<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            validate!(@self.parent.map(|x| x.to_bits())).serialize(serializer)
        }
    }
}
