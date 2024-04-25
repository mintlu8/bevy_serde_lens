#![allow(clippy::approx_constant)]
use bevy_ecs::{component::Component, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::WorldExtension;
use bevy_serde_lens::typetagged::TaggedAny;
use serde::{Deserialize, Serialize};
use serde_json::json;
use bevy_serde_lens::typetagged::AnyTagged;
pub type Any = Box<dyn TaggedAny>;

#[derive(Component, Serialize, Deserialize, TypePath)]
#[serde(transparent)]
pub struct AnyComponent {
    #[serde(with = "AnyTagged")]
    any: Any
}

fn any(v: impl TaggedAny) -> AnyComponent {
    AnyComponent { any : Box::new(v) }
}

#[derive(Debug, Serialize, Deserialize, TypePath)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8
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
    world.register_deserialize_any(|| Ok(Box::new(()) as Any));
    world.register_deserialize_any(|x: i64| Ok(Box::new(x) as Any));
    world.register_deserialize_any(|x: u64| Ok(Box::new(x) as Any));
    world.register_deserialize_any(|x: f64| Ok(Box::new(x) as Any));
    world.register_deserialize_any(|x: bool| Ok(Box::new(x) as Any));
    world.register_deserialize_any(|x: char| Ok(Box::new(x) as Any));
    world.register_deserialize_any(|x: &str| Ok(Box::new(x.to_owned()) as Any));

    world.load::<AnyComponent, _>(json!([69])).unwrap();

    assert_eq!(world.entities().len(), 1);
    let value = world.save::<AnyComponent, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([{"u64": 69}]));
    world.despawn_bound_objects::<AnyComponent>();
    
    world.load::<AnyComponent, _>(json!([-3, false, 72, 0, 3.14, -1.41, "crab", null])).unwrap();

    assert_eq!(world.entities().len(), 8);

    let value = world.save::<AnyComponent, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, json!([
        {"i64": -3},
        {"bool": false},
        {"u64": 72},
        {"u64": 0},
        {"f64": 3.14},
        {"f64": -1.41},
        {"String": "crab"},
        {"()": null},
    ]));

    world.despawn_bound_objects::<AnyComponent>();

    world.register_typetag::<Any, Color>();
    world.register_typetag::<Any, Character>();

    world.load::<AnyComponent, _>(json!([
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
    ])).unwrap();


    let value = world.save::<AnyComponent, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, json!([
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
    ]));
}