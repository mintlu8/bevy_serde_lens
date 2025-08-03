#![cfg(feature = "linkme")]

use bevy_ecs::{component::Component, world::World};
use bevy_reflect::{DynamicTypePath, TypePath};
use bevy_serde_lens::{
    WorldExtension, link_deserializer,
    typetagged::{ErasedObject, TypeTagged},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub trait MyAny: DynamicTypePath + Send + Sync {
    fn as_ser(&self) -> &dyn erased_serde::Serialize;
}

impl<T> MyAny for T
where
    T: DynamicTypePath + erased_serde::Serialize + Send + Sync,
{
    fn as_ser(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

impl<T: MyAny + 'static> From<T> for Box<dyn MyAny> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl ErasedObject for Box<dyn MyAny> {
    fn name(&self) -> impl AsRef<str> {
        self.reflect_short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ser()
    }
}

link_deserializer!(i32 => Box<dyn MyAny>);
link_deserializer!(f32 => Box<dyn MyAny>);

#[derive(Component, TypePath, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AnyComponent(#[serde(with = "TypeTagged")] Box<dyn MyAny>);

#[test]
pub fn test() {
    let mut world = World::new();
    world.spawn(AnyComponent(Box::new(1i32)));
    world.spawn(AnyComponent(Box::new(1.0f32)));

    let value = world
        .save::<AnyComponent, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(
        value,
        json!([
            {"i32": 1},
            {"f32": 1.0},
        ])
    );
}
