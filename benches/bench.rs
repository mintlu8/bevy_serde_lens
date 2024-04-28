use std::sync::{Arc, RwLock};
use bevy_app::App;
use bevy_ecs::{component::Component, world::World};
use bevy_reflect::{Reflect, TypeRegistration, TypeRegistry, TypeRegistryArc};
use bevy_scene::DynamicScene;
use bevy_serde_lens::{ScopedDeserializeLens, WorldExtension};
use criterion::{criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect)]
pub struct Character(String);

fn rand_strings(n: usize) -> Vec<Character> {
    (0..n).map(|_| Character("1231313".to_owned())).collect()
}

pub fn bench_ser_strings(c: &mut Criterion) {
    let strings = rand_strings(1000);
    let mut world = World::new();
    world.spawn_batch(strings.iter().cloned());
    let mut world2 = App::new();
    world2.world.spawn_batch(strings.iter().cloned());
    world2.register_type::<Character>();
    let dynamic_scene = DynamicScene::from_world(&world2.world);
    let mut registry = TypeRegistry::new();
    registry.add_registration(TypeRegistration::of::<Character>());
    let registry = TypeRegistryArc {
        internal: Arc::new(RwLock::new(registry))
    };
    c.bench_function("postcard_strings_vec", |b|{
        b.iter(||postcard::to_allocvec(&strings).unwrap());
    });
    c.bench_function("postcard_strings_serde_lens", |b|{
        b.iter(||postcard::to_allocvec(&world.serialize_lens::<Character>()).unwrap());
    });
    c.bench_function("json_strings_vec", |b|{
        b.iter(||serde_json::to_string(&strings).unwrap());
    });
    c.bench_function("json_strings_serde_lens", |b|{
        b.iter(||serde_json::to_string(&world.serialize_lens::<Character>()).unwrap());
    });
    c.bench_function("ron_strings_vec", |b|{
        b.iter(||ron::to_string(&strings).unwrap());
    });
    c.bench_function("ron_strings_serde_lens", |b|{
        b.iter(||ron::to_string(&world.serialize_lens::<Character>()).unwrap());
    });
    c.bench_function("ron_from_dynamic_scene", |b|{
        b.iter(||dynamic_scene.serialize_ron(&registry));
    });
    c.bench_function("ron_construct_dynamic_scene", |b|{
        b.iter(||DynamicScene::from_world(&world2.world).serialize_ron(&registry));
    });
}


pub fn bench_de_strings(c: &mut Criterion) {
    let strings = rand_strings(100);
    let mut world = World::new();
    let postcard = postcard::to_allocvec(&strings).unwrap();
    let json = serde_json::to_string(&strings).unwrap();
    let ron = ron::to_string(&strings).unwrap();
    c.bench_function("postcard_strings_de", |b|{
        b.iter(||world.scoped_deserialize_lens(|| {
            let _ = postcard::from_bytes::<ScopedDeserializeLens<Character>>(&postcard).unwrap();
        }));
    });
    c.bench_function("json_strings_de", |b|{
        b.iter(||world.scoped_deserialize_lens(|| {
            let _ = serde_json::from_str::<ScopedDeserializeLens<Character>>(&json).unwrap();
        }));
    });
    c.bench_function("ron_strings_de", |b|{
        b.iter(||world.scoped_deserialize_lens(|| {
            let _ = ron::from_str::<ScopedDeserializeLens<Character>>(&ron).unwrap();
        }));
    });
}

criterion_group!(benches, 
    bench_ser_strings,
    bench_de_strings,
);
criterion_main!(benches);
