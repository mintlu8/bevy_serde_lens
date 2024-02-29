use std::{any::type_name, marker::PhantomData};

use bevy_ecs::{entity::Entity, world::World};
use bevy_hierarchy::{BuildWorldChildren, Children};
use itertools::Itertools;

use crate::{BevyObject, BindBevyObject, BoxError, Error, WorldUtil};

#[allow(unused)]
use bevy_ecs::component::Component;

/// Extractor for casting a [`Component`] to its bound [`BevyObject`].
pub type Object<T> = <T as BindBevyObject>::BevyObject;

/// Extractor that allows a [`BevyObject`] to be missing.
pub struct Maybe<T>(PhantomData<T>);

impl<T> BevyObject for Maybe<T> where T: BevyObject {
    type Ser<'t> = Option<T::Ser<'t>> where T: 't;
    type De<'de> = Option<T::De<'de>>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        Ok(Some(T::to_ser(world, entity)?))
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let entity = world.spawn(()).id();
        let Some(de) = de else {return Ok(())};
        T::from_de(world, entity, de)?;
        world.entity_mut_ok(parent)?.add_child(entity);
        Ok(())
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// instead of the entity itself. 
///
/// This will iterate through all children
/// to validate uniqueness. [`ChildUnchecked`] is a non-checking
/// alternative. Alternatively use [`ChildList`] for a list of objects.
///
/// # Errors
///
/// When more than one item is found.
pub struct Child<T>(T);

impl<T> BevyObject for Child<T> where T: BevyObject {
    type Ser<'t> = T::Ser<'t> where T: 't;
    type De<'de> = T::De<'de>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {return Ok(None);};
        match children.iter()
            .filter_map(|entity| T::to_ser(world, *entity).transpose())
            .at_most_one() 
        {
            Ok(None) => Ok(None),
            Ok(Some(Ok(item))) => Ok(Some(item)),
            Ok(Some(Err(err))) => Err(err),
            Err(mut iter) => match iter.find_map(Result::err) {
                Some(err) => Err(err),
                None => Err(Error::MoreThenOne { 
                    parent: entity,
                    ty: type_name::<T>()
                }.boxed()),
            }
        }
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let entity = world.spawn(()).id();
        T::from_de(world, entity, de)?;
        world.entity_mut(parent).add_child(entity);
        Ok(())
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// instead of the entity itself. 
///
/// This will find the first item and
/// may discard duplicate entities. 
/// Alternatively use [`ChildList`] for a list of objects.
pub struct ChildUnchecked<T>(T);

impl<T> BevyObject for ChildUnchecked<T> where T: BevyObject {
    type Ser<'t> = T::Ser<'t> where T: 't;
    type De<'de> = T::De<'de>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {return Ok(None);};
        match children.iter().find_map(|entity| T::to_ser(world, *entity).transpose()) {
            Some(Ok(result)) => Ok(Some(result)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let entity = world.spawn(()).id();
        T::from_de(world, entity, de)?;
        world.entity_mut(parent).add_child(entity);
        Ok(())
    }
}

/// Extractor for matching [`BevyObject`]s on a [`Children`].
///
/// This serializes similar to a `Vec`.
pub struct ChildList<T>(T);

fn flatten<T>(res: Result<Option<T>, BoxError>, ty: &'static str) -> Result<T, BoxError>{
    match res {
        Ok(Some(item)) => Ok(item),
        Ok(None) => Err(Error::ChildrenReturnedNone { ty }.boxed()),
        Err(e) => Err(e),
    }
}

impl<T> BevyObject for ChildList<T> where T: BevyObject {
    type Ser<'t> = Vec<T::Ser<'t>> where T: 't;
    type De<'de> = T::De<'de>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {
            return Ok(Some(Vec::new()));
        };
        children.iter()
            .map(|entity| flatten(T::to_ser(world, *entity), type_name::<T>()))
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let entity = world.spawn(()).id();
        T::from_de(world, entity, de)?;
        world.entity_mut(parent).add_child(entity);
        Ok(())
    }
}
