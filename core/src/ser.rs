use crate::{ENTITY, WORLD};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::ReadOnlyQueryData;
use bevy_ecs::resource::Resource;
use bevy_ecs::world::{EntityRef, World};
use serde::ser::Error as SError;
use serde::Serializer;
use std::convert::Infallible;
use std::fmt::Display;

/// Useful commands for serialization.
pub struct SerUtils(Infallible);

impl SerUtils {
    /// Run a function on a read only reference to [`World`].
    ///
    /// # Errors
    ///
    /// * If used outside of a `Serialize` implementation.
    /// * If used outside `bevy_serde_lens`.
    #[inline(always)]
    pub fn with_world<S: Serializer, T>(f: impl FnOnce(&World) -> T) -> Result<T, S::Error> {
        if !WORLD.is_set() {
            Err(SError::custom(
                "Cannot serialize outside a `serialize` scope.",
            ))
        } else {
            Ok(WORLD.with(f))
        }
    }

    /// Obtain the current [`Entity`] in `bevy_serde_lens`.
    ///
    /// # Errors
    ///
    /// * If used outside `bevy_serde_lens`.
    #[inline(always)]
    pub fn current_entity<S: Serializer>() -> Result<Entity, S::Error> {
        ENTITY
            .get()
            .ok_or_else(|| SError::custom("No active entity in serialization found."))
    }

    pub fn with_entity_ref<S: Serializer, T>(
        f: impl FnOnce(EntityRef) -> T,
    ) -> Result<T, S::Error> {
        let Some(entity) = ENTITY.get() else {
            return Err(SError::custom("No active entity in serialization found."));
        };
        if !WORLD.is_set() {
            return Err(SError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
        WORLD.with(|world| {
            world
                .get_entity(entity)
                .map(f)
                .map_err(|_| SError::custom("Entity missing."))
        })
    }

    pub fn with_query<C: ReadOnlyQueryData, S: Serializer, T>(
        f: impl FnOnce(C::Item<'_>) -> T,
    ) -> Result<T, S::Error> {
        let Some(entity) = ENTITY.get() else {
            return Err(SError::custom("No active entity in serialization found."));
        };
        if !WORLD.is_set() {
            return Err(SError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
        WORLD.with(|world| {
            world
                .get_entity(entity)
                .map_err(|_| SError::custom("Entity missing."))?
                .get_components::<C>()
                .map(f)
                .ok_or_else(|| SError::custom("One or more component missing."))
        })
    }

    pub fn with_component<C: Component, S: Serializer, T>(
        f: impl FnOnce(&C) -> T,
    ) -> Result<T, S::Error> {
        let Some(entity) = ENTITY.get() else {
            return Err(SError::custom("No active entity in serialization found."));
        };
        if !WORLD.is_set() {
            return Err(SError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
        WORLD.with(|world| {
            world
                .get_entity(entity)
                .map_err(|_| SError::custom("Entity missing."))?
                .get::<C>()
                .map(f)
                .ok_or_else(|| SError::custom("One or more component missing."))
        })
    }

    pub fn with_resource<R: Resource, S: Serializer, T>(
        f: impl FnOnce(&R) -> T,
    ) -> Result<T, S::Error> {
        if !WORLD.is_set() {
            return Err(SError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
        WORLD.with(|world| {
            world
                .get_resource::<R>()
                .map(f)
                .ok_or_else(|| SError::custom("Resource missing."))
        })
    }

    pub fn with_non_send_resource<R: 'static, S: Serializer, T>(
        f: impl FnOnce(&R) -> T,
    ) -> Result<T, S::Error> {
        if !WORLD.is_set() {
            return Err(SError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
        WORLD.with(|world| {
            world
                .get_non_send_resource::<R>()
                .map(f)
                .ok_or_else(|| SError::custom("Resource missing."))
        })
    }

    pub fn error<S: Serializer>(string: impl Display) -> S::Error {
        SError::custom(string)
    }
}
