#![allow(clippy::upper_case_acronyms)]
use bevy_ecs::{component::Component, query::With, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{bind_object, DefaultInit, WorldExtension};
use serde_json::json;

#[derive(Component, TypePath, Default)]
pub struct A(char);

bind_object!(
    pub struct B as A {
        a: DefaultInit<A>,
    }
);


bind_object!(
    pub struct C as A {
        #[serde(default)]
        a: DefaultInit<A>,
    }
);

#[test]
pub fn test() {
    let mut world = World::new();
    world.load::<C, _>(json!([{}])).unwrap();
    let mut  query = world.query_filtered::<(), With<A>>();
    assert!(query.get_single(&world).is_ok());

    world.despawn_bound_objects::<C>();
    assert!(world.entities().is_empty());
    assert!(query.get_single(&world).is_err());

    world.despawn_bound_objects::<C>();
    assert!(world.load::<B, _>(json!([{}])).is_err());
    assert!(world.entities().is_empty());
    assert!(query.get_single(&world).is_err());
    
    world.load::<C, _>(json!([{"a": null}])).unwrap();
    assert!(query.get_single(&world).is_ok());

    world.despawn_bound_objects::<C>();
    world.load::<B, _>(json!([{"a": null}])).unwrap();
    assert!(query.get_single(&world).is_ok());
}