use std::any::Any;

use bevy_ecs::{component::Component, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{Adjacent, BevyObject, DeUtils, SerializeAdjacent, WorldExtension};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;


#[derive(Debug, Component, Serialize, Deserialize, TypePath)]
#[serde(transparent)]
pub struct SerializeInt(u8);

#[derive(Component)]
pub struct Anything(Box<dyn Any + Send + Sync>);

impl SerializeAdjacent<&Anything> for &SerializeInt {
    fn name() -> &'static str {
        "Anything"
    }
    
    fn serialize_adjacent<S: Serializer>(
        this: &&SerializeInt,
        other: &&Anything,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match this.0 {
            1 => other.0.downcast_ref::<u8>().unwrap().serialize(serializer),
            2 => other.0.downcast_ref::<u16>().unwrap().serialize(serializer),
            4 => other.0.downcast_ref::<u32>().unwrap().serialize(serializer),
            8 => other.0.downcast_ref::<u64>().unwrap().serialize(serializer),
            _ => Err(serde::ser::Error::custom("Expected 1, 2, 4, 8"))
        }
    }
    
    fn deserialize_adjacent<'de, D: serde::Deserializer<'de>>(
        this: &&SerializeInt,
        deserializer: D,
    ) -> Result<(), D::Error> {
        match this.0 {
            1 => DeUtils::insert::<D>(Anything(Box::new(u8::deserialize(deserializer)?))),
            2 => DeUtils::insert::<D>(Anything(Box::new(u16::deserialize(deserializer)?))),
            4 => DeUtils::insert::<D>(Anything(Box::new(u32::deserialize(deserializer)?))),
            8 => DeUtils::insert::<D>(Anything(Box::new(u64::deserialize(deserializer)?))),
            _ => Err(serde::de::Error::custom("Expected 1, 2, 4, 8"))
        }
    }
}

#[derive(BevyObject)]
struct Ints {
    len: SerializeInt,
    value: Adjacent<&'static SerializeInt, &'static Anything>,
}

#[test]
fn adjacent_test() {
    let mut world = World::new();

    world.spawn((
        SerializeInt(1),
        Anything(Box::new(4u8))
    ));
    world.spawn((
        SerializeInt(2),
        Anything(Box::new(6u16))
    ));
    world.spawn((
        SerializeInt(4),
        Anything(Box::new(12u32))
    ));
    world.spawn((
        SerializeInt(8),
        Anything(Box::new(82u64))
    ));

    let value = world.save::<Ints, _>(serde_json::value::Serializer).unwrap();
    let expected_result = json!([
        {
            "len": 1,
            "value": 4,
        },
        {
            "len": 2,
            "value": 6,
        },
        {
            "len": 4,
            "value": 12,
        },
        {
            "len": 8,
            "value": 82,
        },
    ]);
    assert_eq!(value, expected_result)
}