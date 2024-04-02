//! Module for serializing [`Entity`]s and [`EntitySmartPointer`]s.

use std::marker::PhantomData;
use bevy_ecs::{bundle::Bundle, entity::Entity, world::World};
use ref_cast::RefCast;
use crate::{BevyObject, Convert, Error, SerdeProject, WorldAccess};

/// Projection of [`Entity`] or an [`EntitySmartPointer`] that is to be spawned independently.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct EntityPointer<B: BevyObject, E: EntitySmartPointer<B>=Entity>(
    pub E,
    PhantomData<B>
);

impl<B: BevyObject, E: EntitySmartPointer<B,Pointee = ()>> EntityPointer<B, E> {
    pub fn new(entity: Entity) -> Self{
        EntityPointer(E::from_entity(entity), PhantomData)
    }
}

impl<B: BevyObject, E: EntitySmartPointer<B>> Convert<E> for EntityPointer<B, E> {
    fn ser(input: &E) -> &Self {
        EntityPointer::ref_cast(input)
    }

    fn de(self) -> E {
        self.0
    }
}

impl<B: BevyObject, E: EntitySmartPointer<B>> EntityPointer<B, E> {
    pub fn new_pointee(entity: Entity) -> (Self, E::Pointee){
        let mut smart_ptr = E::from_entity(entity);
        let pointee = smart_ptr.inject_pointee();
        (EntityPointer(smart_ptr, PhantomData), pointee)
    }
}

impl<B: BevyObject, E: EntitySmartPointer<B>> SerdeProject for EntityPointer<B, E> {
    type Ctx = WorldAccess;
    type Ser<'t> = B::Ser<'t> where B: 't, E: 't;
    type De<'de> = B::De<'de>;

    fn to_ser<'t>(&'t self, world: &<Self::Ctx as crate::FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, Box<crate::Error>> {
        match B::to_ser(world, self.0.get_entity()) {
            Ok(Some(result)) => Ok(result),
            Ok(None) => Err(Box::new(Error::EntityMissing(self.0.get_entity()))),
            Err(e) => Err(e)
        }
    }

    fn from_de(world: &mut <Self::Ctx as crate::FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, Box<crate::Error>> {
        Ok(EntityPointer(E::find_or_init(world, de)?, PhantomData))
    }
}

/// A trait that enables building `Rc` like relation between entities.
pub trait EntitySmartPointer<B: BevyObject>: Sized {
    type Pointee: Bundle;
    fn from_entity(entity: Entity) -> Self;
    fn get_entity(&self) -> Entity;
    fn inject_pointee(&mut self) -> Self::Pointee;

    fn find_or_init(world: &mut World, de: <B as BevyObject>::De<'_>) -> Result<Self, Box<Error>> {
        let entity = world.spawn_empty().id();
        B::from_de(world, entity, de)?;
        let mut result = Self::from_entity(entity);
        world.entity_mut(entity).insert(result.inject_pointee());
        Ok(result)
    }
}

impl<B: BevyObject> EntitySmartPointer<B> for Entity {
    type Pointee = ();
    fn from_entity(entity: Entity) -> Self {
        entity
    }

    fn get_entity(&self) -> Entity {
        *self
    }

    fn inject_pointee(&mut self) -> Self::Pointee {}
}
