//! Module for serializing [`Entity`]s and [`EntityPointer`]s.

use std::marker::PhantomData;
use bevy_ecs::{bundle::Bundle, entity::Entity};
use ref_cast::RefCast;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::{with_world_mut, BevyObject, ENTITY};

/// Projection of [`Entity`] or an [`EntityPointer`] that is to be spawned independently.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct EntityPtr<B: BevyObject, E: EntityPointer<B>=Entity>(
    pub E,
    PhantomData<B>
);

impl<B: BevyObject, E: EntityPointer<B,Pointee = ()>> EntityPtr<B, E> {
    pub fn new(entity: Entity) -> Self{
        EntityPtr(E::from_entity(entity), PhantomData)
    }
}

impl<B: BevyObject> EntityPtr<B> {
    /// Serialize with [`EntityPointer`].
    pub fn serialize<T: EntityPointer<B>, S: serde::Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error> {
        EntityPtr::ref_cast(item).serialize(serializer)
    }

    /// Deserialize with [`EntityPointer`].
    pub fn deserialize<'de, T: EntityPointer<B>, D: serde::Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        <EntityPtr<B, T>>::deserialize(deserializer).map(|x| x.0)
    }
}

impl<B: BevyObject, E: EntityPointer<B>> EntityPtr<B, E> {
    pub fn new_pointee(entity: Entity) -> (Self, E::Pointee){
        let mut smart_ptr = E::from_entity(entity);
        let pointee = smart_ptr.inject_pointee();
        (EntityPtr(smart_ptr, PhantomData), pointee)
    }
}

impl<B: BevyObject, E: EntityPointer<B>> Serialize for EntityPtr<B, E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ENTITY.set(&self.0.get_entity(), ||{
            B::init().serialize(serializer)
        })
    }
}

impl<'de, B: BevyObject, E: EntityPointer<B>> Deserialize<'de> for EntityPtr<B, E> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entity = with_world_mut::<_, D>(|world| world.spawn_empty().id())?;
        ENTITY.set(&entity, ||B::Object::deserialize(deserializer))?;
        let mut item = E::from_entity(entity);
        with_world_mut::<_, D>(|world| {
            world.entity_mut(entity).insert(item.inject_pointee());
        })?;
        Ok(Self(item, PhantomData))
    }
}

/// A trait that enables building `Rc` like relation between entities.
pub trait EntityPointer<B: BevyObject>: Sized {
    type Pointee: Bundle;
    fn from_entity(entity: Entity) -> Self;
    fn get_entity(&self) -> Entity;
    fn inject_pointee(&mut self) -> Self::Pointee;
}

impl<B: BevyObject> EntityPointer<B> for Entity {
    type Pointee = ();
    fn from_entity(entity: Entity) -> Self {
        entity
    }

    fn get_entity(&self) -> Entity {
        *self
    }

    fn inject_pointee(&mut self) -> Self::Pointee {}
}

// `EntityRc` could be a part of this crate but we do not 
// require or provide systems by design. See tests for a implementation.