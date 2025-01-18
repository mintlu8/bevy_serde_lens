#![allow(clippy::approx_constant)]
use bevy_ecs::{component::Component, world::World};
use bevy_reflect::{DynamicTypePath, TypePath};
use bevy_serde_lens::typetagged::ErasedObject;
use bevy_serde_lens::typetagged::{AnyOrTagged, DeserializeError};
use bevy_serde_lens::WorldExtension;
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

macro_rules! impl_deserialize_any {
    ($($ty: ty, $fn: ident,)*) => {
        $(
            fn $fn(value: $ty) -> Result<Self, DeserializeError> {
                Ok(Box::new(value))
            }
        )*
    };
}

impl ErasedObject for Box<dyn MyAny> {
    fn name(&self) -> impl AsRef<str> {
        self.reflect_short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ser()
    }

    impl_deserialize_any! {
        bool, deserialize_bool,
        i64, deserialize_int,
        u64, deserialize_uint,
        f64, deserialize_float,
        char, deserialize_char,
    }

    fn deserialize_unit() -> Result<Self, DeserializeError> {
        Ok(Box::new(()))
    }

    fn deserialize_string(value: &str) -> Result<Self, DeserializeError> {
        Ok(Box::new(value.to_owned()))
    }
}

#[derive(Component, Serialize, Deserialize, TypePath)]
#[serde(transparent)]
pub struct AnyComponent {
    #[serde(with = "AnyOrTagged")]
    any: Box<dyn MyAny>,
}

fn any(v: impl MyAny + 'static) -> AnyComponent {
    AnyComponent { any: Box::new(v) }
}

#[derive(Debug, Serialize, Deserialize, TypePath)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize, TypePath)]
pub struct Character {
    name: String,
    age: u64,
    gender: bool,
}

#[test]
pub fn test() {
    let mut world = World::new();

    world.load::<AnyComponent, _>(json!([69])).unwrap();

    assert_eq!(world.entities().len(), 1);
    let value = world
        .save::<AnyComponent, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(value, json!([{"u64": 69}]));
    world.despawn_bound_objects::<AnyComponent>();

    world
        .load::<AnyComponent, _>(json!([-3, false, 72, 0, 3.14, -1.41, "crab", null]))
        .unwrap();

    assert_eq!(world.entities().len(), 8);

    let value = world
        .save::<AnyComponent, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(
        value,
        json!([
            {"i64": -3},
            {"bool": false},
            {"u64": 72},
            {"u64": 0},
            {"f64": 3.14},
            {"f64": -1.41},
            {"String": "crab"},
            {"()": null},
        ])
    );

    world.despawn_bound_objects::<AnyComponent>();

    world.register_typetag::<Box<dyn MyAny>, Color>();
    world.register_typetag::<Box<dyn MyAny>, Character>();

    world
        .load::<AnyComponent, _>(json!([
            3,
            {
                "Color": {
                    "r": 32,
                    "g": 252,
                    "b": 144
                }
            },
            4.5,
            {
                "Character": {
                    "name": "Carl",
                    "age": 44,
                    "gender": false
                }
            },
            true
        ]))
        .unwrap();

    let value = world
        .save::<AnyComponent, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(
        value,
        json!([
            {"u64": 3},
            {
                "Color": {
                    "r": 32,
                    "g": 252,
                    "b": 144
                }
            },
            {"f64": 4.5},
            {
                "Character": {
                    "name": "Carl",
                    "age": 44,
                    "gender": false
                }
            },
            {"bool": true},
        ])
    );
}
