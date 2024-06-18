//! The core world access module of `bevy_serde_lens`.
//!
//! Crates that depend on `bevy_serde_lens` for serialization
//! should depend on this crate for world access
//! since this tracks `bevy` versions instead of
//! `bevy_serde_lens` versions.
//!
//! # Versioning
//!
//! There should be one release per bevy major version with the same version number.

use std::{cell::Cell, fmt::Display};

use bevy_ecs::{entity::Entity, world::World};

scoped_tls_hkt::scoped_thread_local!(
    static WORLD: World
);

scoped_tls_hkt::scoped_thread_local!(
    static mut WORLD_MUT: World
);

thread_local! {
    static ENTITY: Cell<Option<Entity>> = const {Cell::new(None)}
}

#[derive(Debug)]
pub struct Error(&'static str);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Error {}

/// Run a function on a read only reference to [`World`].
///
/// # Errors
///
/// * If used outside of a `Serialize` implementation.
/// * If used outside `bevy_serde_lens`.
pub fn with_world<T>(f: impl FnOnce(&World) -> T) -> Result<T, Error> {
    if !WORLD.is_set() {
        Err(Error("Cannot serialize outside the `save` scope."))
    } else {
        Ok(WORLD.with(f))
    }
}

/// Run a function on a mutable only reference to [`World`].
///
/// # Errors
///
/// * If used outside of a [`Deserialize`] implementation.
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
pub fn with_world_mut<T>(f: impl FnOnce(&mut World) -> T) -> Result<T, Error> {
    if !WORLD_MUT.is_set() {
        Err(Error("Cannot deserialize outside the `load` scope."))
    } else {
        Ok(WORLD_MUT.with(f))
    }
}

pub fn current_entity() -> Result<Entity, Error> {
    ENTITY.get().ok_or(Error("No active entity found."))
}

#[doc(hidden)]
pub mod __private {
    use bevy_ecs::{entity::Entity, world::World};

    use crate::{ENTITY, WORLD, WORLD_MUT};

    #[inline(always)]
    pub fn ser_scope<T>(world: &World, f: impl FnOnce() -> T) -> T {
        WORLD.set(world, f)
    }

    #[inline(always)]
    pub fn de_scope<T>(world: &mut World, f: impl FnOnce() -> T) -> T {
        WORLD_MUT.set(world, f)
    }

    struct DeferredEntity(Option<Entity>);

    impl Drop for DeferredEntity {
        fn drop(&mut self) {
            ENTITY.set(self.0)
        }
    }

    #[inline(always)]
    pub fn entity_scope<T>(entity: Entity, f: impl FnOnce() -> T) -> T {
        let _entity = DeferredEntity(ENTITY.replace(Some(entity)));
        f()
    }
}
