use bevy_ecs::{system::Resource, world::{Mut, World}};
use crate::{BoxError, Error, SerdeProject, WorldUtil};

/// Represents no context.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoContext;


/// Represents `&World` and `&mut World`.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldAccess;

/// Convenience trait for fetching something from the [`World`].
///
/// Standard implementations [`NoContext`], [`Resource`] and [`WorldAccess`] should be good for most use cases.
pub trait FromWorldAccess {
    type Ref<'t>;
    type Mut<'t>;

    fn from_world(world: &World) -> Result<Self::Ref<'_>, Box<Error>>;
    fn from_world_mut(world: &mut World) -> Result<Self::Mut<'_>, Box<Error>>;
}

impl FromWorldAccess for NoContext {
    type Ref<'t> = ();
    type Mut<'t> = ();

    fn from_world(_: &World) -> Result<Self::Ref<'_>, BoxError> {
        Ok(())
    }

    fn from_world_mut(_: &mut World) -> Result<Self::Mut<'_>, BoxError> {
        Ok(())
    }
}


impl FromWorldAccess for WorldAccess {
    type Ref<'t> = &'t World;
    type Mut<'t> = &'t mut World;

    fn from_world(w: &World) -> Result<Self::Ref<'_>, BoxError> {
        Ok(w)
    }

    fn from_world_mut(w: &mut World) -> Result<Self::Mut<'_>, BoxError> {
        Ok(w)
    }
}

impl<T> FromWorldAccess for T where T: Resource {
    type Ref<'t> = &'t T;
    type Mut<'t> = Mut<'t, T>;

    fn from_world(world: &World) -> Result<Self::Ref<'_>, BoxError> {
        world.resource_ok::<T>()
    }

    fn from_world_mut(world: &mut World) -> Result<Self::Mut<'_>, BoxError> {
        world.resource_mut_ok::<T>()
    }
}

/// Utility for implementing [`SerdeProject`].
pub fn from_world<T: SerdeProject>(world: &World) -> Result<<T::Ctx as FromWorldAccess>::Ref<'_>, BoxError> {
    <T::Ctx as FromWorldAccess>::from_world(world)
}

/// Utility for implementing [`SerdeProject`].
pub fn from_world_mut<T: SerdeProject>(world: &mut World) -> Result<<T::Ctx as FromWorldAccess>::Mut<'_>, BoxError> {
    <T::Ctx as FromWorldAccess>::from_world_mut(world)
}