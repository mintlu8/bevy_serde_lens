use crate::{ENTITY, WORLD_MUT};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::{Component, Mutable};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::ReadOnlyQueryData;
use bevy_ecs::resource::Resource;
use bevy_ecs::world::{EntityWorldMut, Mut, World};
use serde::de::{Error as DError, SeqAccess};
use serde::Deserializer;
use std::convert::Infallible;
use std::fmt::Display;

/// Useful commands for deserialization.
pub struct DeUtils(Infallible);

macro_rules! validate_world {
    () => {
        if !WORLD_MUT.is_set() {
            return Err(DError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
    };
}

impl DeUtils {
    /// Run a function on a mutable only reference to [`World`].
    ///
    /// # Errors
    ///
    /// * If used outside of a `Deserialize` implementation.
    /// * If used outside `bevy_serde_lens`.
    /// * If used in a nested manner, as that is a violation to rust's aliasing rule.
    ///
    /// ```
    /// with_world_mut(|| {
    ///     // panics here
    ///     with_world_mut(|| {
    ///         ..
    ///     })
    /// })
    /// ```
    #[inline(always)]
    pub fn with_world_mut<'de, D: Deserializer<'de>, T>(
        f: impl FnOnce(&mut World) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        Ok(WORLD_MUT.with(f))
    }

    #[doc(hidden)]
    pub fn with_world_mut_seq<'de, S: SeqAccess<'de>, T>(
        f: impl FnOnce(&mut World) -> T,
    ) -> Result<T, S::Error> {
        validate_world!();
        Ok(WORLD_MUT.with(f))
    }

    /// Obtain the current [`Entity`] in `bevy_serde_lens`.
    ///
    /// # Errors
    ///
    /// * If used outside `bevy_serde_lens`.
    #[inline(always)]
    pub fn current_entity<'de, D: Deserializer<'de>>() -> Result<Entity, D::Error> {
        ENTITY
            .get()
            .ok_or_else(|| DError::custom("No active entity in deserialization found."))
    }

    pub fn with_entity_mut<'de, D: Deserializer<'de>, T>(
        f: impl FnOnce(EntityWorldMut) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        let Some(entity) = ENTITY.get() else {
            return Err(DError::custom("No active entity in serialization found."));
        };
        WORLD_MUT.with(|world| {
            world
                .get_entity_mut(entity)
                .map(f)
                .map_err(|_| DError::custom("Entity missing."))
        })
    }

    pub fn with_query<'de, C: ReadOnlyQueryData, D: Deserializer<'de>, T>(
        f: impl FnOnce(C::Item<'_>) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        let Some(entity) = ENTITY.get() else {
            return Err(DError::custom("No active entity in serialization found."));
        };
        WORLD_MUT.with(|world| {
            world
                .get_entity(entity)
                .map_err(|_| DError::custom("Entity missing."))?
                .get_components::<C>()
                .map(f)
                .ok_or_else(|| DError::custom("One or more component missing."))
        })
    }

    pub fn with_component<'de, C: Component, D: Deserializer<'de>, T>(
        f: impl FnOnce(&C) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        let Some(entity) = ENTITY.get() else {
            return Err(DError::custom("No active entity in serialization found."));
        };
        WORLD_MUT.with(|world| {
            world
                .get_entity(entity)
                .map_err(|_| DError::custom("Entity missing."))?
                .get::<C>()
                .map(f)
                .ok_or_else(|| DError::custom("Component missing."))
        })
    }

    pub fn with_component_mut<'de, C: Component<Mutability = Mutable>, D: Deserializer<'de>, T>(
        f: impl FnOnce(Mut<C>) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        let Some(entity) = ENTITY.get() else {
            return Err(DError::custom("No active entity in serialization found."));
        };
        WORLD_MUT.with(|world| {
            world
                .get_entity_mut(entity)
                .map_err(|_| DError::custom("Entity missing."))?
                .get_mut::<C>()
                .map(f)
                .ok_or_else(|| DError::custom("Component missing."))
        })
    }

    pub fn with_resource<'de, R: Resource, D: Deserializer<'de>, T>(
        f: impl FnOnce(&R) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        WORLD_MUT.with(|world| {
            world
                .get_resource::<R>()
                .map(f)
                .ok_or_else(|| DError::custom("Resource missing."))
        })
    }

    pub fn with_resource_mut<'de, R: Resource, D: Deserializer<'de>, T>(
        f: impl FnOnce(Mut<R>) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        WORLD_MUT.with(|world| {
            world
                .get_resource_mut::<R>()
                .map(f)
                .ok_or_else(|| DError::custom("Resource missing."))
        })
    }

    pub fn with_non_send_resource<'de, R: 'static, D: Deserializer<'de>, T>(
        f: impl FnOnce(&R) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        WORLD_MUT.with(|world| {
            world
                .get_non_send_resource::<R>()
                .map(f)
                .ok_or_else(|| DError::custom("Resource missing."))
        })
    }

    pub fn with_non_send_resource_mut<'de, R: 'static, D: Deserializer<'de>, T>(
        f: impl FnOnce(Mut<R>) -> T,
    ) -> Result<T, D::Error> {
        validate_world!();
        WORLD_MUT.with(|world| {
            world
                .get_non_send_resource_mut::<R>()
                .map(f)
                .ok_or_else(|| DError::custom("Resource missing."))
        })
    }

    pub fn insert<'de, D: Deserializer<'de>>(bundle: impl Bundle) -> Result<(), D::Error> {
        let Some(entity) = ENTITY.get() else {
            return Err(DError::custom("No active entity in serialization found."));
        };
        if !WORLD_MUT.is_set() {
            return Err(DError::custom(
                "Cannot deserialize outside of a `deserialize` scope.",
            ));
        }
        WORLD_MUT.with(|world| {
            world
                .get_entity_mut(entity)
                .map(|mut x| {
                    x.insert(bundle);
                })
                .map_err(|_| DError::custom("Entity missing."))
        })
    }

    pub fn error<'de, D: Deserializer<'de>>(string: impl Display) -> D::Error {
        DError::custom(string)
    }
}
