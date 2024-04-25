use std::borrow::Cow;

use bevy_ecs::{component::Component, system::Resource, world::World};
use bevy_serde_lens::{bind_object, interning::{Interner, InterningKey}, Error, SerdeProject, WorldExtension};
use rustc_hash::FxHashMap;
use bevy_serde_lens::interning::Interned;
use serde_json::json;
pub struct Flag(u64);

#[derive(Resource)]
pub struct FlagsServer {
    i2s: Vec<String>,
    s2i: FxHashMap<String, u64>
}

impl Default for FlagsServer {
    fn default() -> Self {
        Self { i2s: vec!["".to_owned()], s2i: Default::default() }
    }
}

impl InterningKey for Flag {
    type Interner = FlagsServer;
}

impl Interner<Flag> for FlagsServer {
    type ValueRef<'t> = String;
    type Value<'de> = Cow<'de, str>;

    fn get(&self, key: &Flag) -> Result<String, Box<Error>> {
        let mut n = 1;
        let mut vec = Vec::new();
        let mut key = key.0;
        while key > 0 {
            if key % 2 == 1 {
                vec.push(self.i2s[n].as_ref());
            }
            n += 1;
            key >>= 1;
        }
        Ok(vec.join("|"))
    }

    fn add(&mut self, value: Self::Value<'_>) -> Result<Flag, Box<Error>> {
        Ok(Flag(value.split('|').map(|s| 
            self.s2i.get(s).copied().unwrap_or_else(|| {
                let val = 1 << (self.i2s.len() - 1);
                self.i2s.push(s.to_owned()); 
                self.s2i.insert(s.to_owned(), val);
                val
            })
        ).fold(0u64, |a, b| a| b)))
    }
}

#[derive(Component, SerdeProject)]
#[serde(transparent)]
pub struct FlagComponent {
    #[serde_project("Interned<Flag>")]
    pub flag: Flag,
}

bind_object!(FlagComponent as "Flag");

#[test]
pub fn test() {
    let mut world = World::new();
    let mut server = FlagsServer::default();
    let flag1 = server.add("red|green|blue".into()).unwrap();
    let flag2 = server.add("yellow|red".into()).unwrap();
    let flag3 = Flag(flag1.0 | flag2.0);
    world.insert_resource(server);
    world.spawn(FlagComponent{
        flag: flag1
    });
    world.spawn(FlagComponent{
        flag: flag2
    });
    world.spawn(FlagComponent{
        flag: flag3
    });
    let value = world.save::<FlagComponent, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        "red|green|blue",
        "red|yellow",
        "red|green|blue|yellow",
    ]));

    world.despawn_bound_objects::<FlagComponent>();
    assert_eq!(world.entities().len(), 0);
    world.load::<FlagComponent, _>(&value).unwrap();
    assert_eq!(world.entities().len(), 3);

    let value = world.save::<FlagComponent, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        "red|green|blue",
        "red|yellow",
        "red|green|blue|yellow",
    ]));

    world.load::<FlagComponent, _>(&json!([
        "green|blue",
        "white|red|black",
    ])).unwrap();


    let value = world.save::<FlagComponent, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        "red|green|blue",
        "red|yellow",
        "red|green|blue|yellow",
        "green|blue",
        "red|white|black",
    ]));

}