//! The core world access module of `bevy_serde_lens`.
//!
//! Crates that depend on `bevy_serde_lens` for serialization
//! should depend on this crate for world access
//! since this tracks `bevy` versions instead of
//! `bevy_serde_lens` versions.

use std::cell::Cell;

use bevy_ecs::{entity::Entity, world::World};

mod de;
mod scope;
mod ser;

pub use de::DeUtils;
pub use scope::ScopeUtils;
pub use ser::SerUtils;

scoped_tls_hkt::scoped_thread_local!(
    static WORLD: World
);

scoped_tls_hkt::scoped_thread_local!(
    static mut WORLD_MUT: World
);

thread_local! {
    static ENTITY: Cell<Option<Entity>> = const {Cell::new(None)}
}
