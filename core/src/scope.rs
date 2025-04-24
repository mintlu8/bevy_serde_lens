use std::convert::Infallible;

use bevy_ecs::{entity::Entity, world::World};

use crate::{ENTITY, WORLD, WORLD_MUT};

/// Support for creating custom `bevy_serde_lens` scopes outside of `bevy_serde_lens`.
pub struct ScopeUtils(Infallible);

struct DespawnEntity(Option<Entity>);

impl Drop for DespawnEntity {
    fn drop(&mut self) {
        ENTITY.set(self.0)
    }
}

impl ScopeUtils {
    /// Setup a `serialize` scope.
    #[inline(always)]
    pub fn serialize_scope<T>(world: &World, f: impl FnOnce() -> T) -> T {
        WORLD.set(world, f)
    }

    /// Setup a `deserialize` scope.
    #[inline(always)]
    pub fn deserialize_scope<T>(world: &mut World, f: impl FnOnce() -> T) -> T {
        WORLD_MUT.set(world, f)
    }

    /// Run a closure with `entity` as the current entity.
    #[inline(always)]
    pub fn current_entity_scope<T>(entity: Entity, f: impl FnOnce() -> T) -> T {
        let _entity = DespawnEntity(ENTITY.replace(Some(entity)));
        f()
    }
}
