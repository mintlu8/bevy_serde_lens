//! Module for loading deserializers via [`linkme`].
use bevy_app::Plugin;
use bevy_ecs::world::World;
use bevy_reflect::TypePath;
use serde::de::DeserializeOwned;

use crate::{typetagged::ErasedObject, WorldExtension};

#[doc(hidden)]
pub use linkme::distributed_slice;

#[doc(hidden)]
pub type Func = fn(&mut World);

/// A [`linkme`] slice of all registered deserializers.
#[distributed_slice]
pub static DESERIALIZER_PLUGINS: [fn(&mut World)];

/// Create a function to use with [`DESERIALIZER_PLUGINS`].
pub const fn as_deserialize_plugin<A: ErasedObject, B: Into<A> + TypePath + DeserializeOwned>() -> fn(&mut World) {
    |world| {
        world.register_typetag::<A, B>();
    }
}

/// Plugin that registers all functions linked with [`DESERIALIZER_PLUGINS`].
pub struct LinkDeserializersPlugin;

impl Plugin for LinkDeserializersPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        for f in DESERIALIZER_PLUGINS {
            f(app.world_mut())
        }
    }
}

/// Link types to be registered with [`LinkDeserializersPlugin`].
/// 
/// # Example
/// ```
/// link_deserializer!(Cat => Box<dyn Animal>);
/// link_deserializer!(Cat => {
///     Box<dyn Animal>,
///     Box<dyn GameObject>,
///     Box<dyn Metadata>,
/// });
/// ```
#[macro_export]
macro_rules! link_deserializer {
    ($lhs: ty => $rhs: ty) => {
        const _: () = {
            #[$crate::linking::distributed_slice($crate::linking::DESERIALIZER_PLUGINS)]
            static __DESERIALIZER: $crate::linking::Func = $crate::linking::as_deserialize_plugin::<$rhs, $lhs>();
        };
    };
    ($lhs: ty => {$($rhs: ty),* $(,)?}) => {
        $(
            const _: () = {
                #[$crate::linking::distributed_slice]
                static __DESERIALIZER: $crate::linking::Func = $crate::linking::as_deserialize_plugin::<$rhs, $lhs>();
            };
        )*
    };
}