#![allow(clippy::upper_case_acronyms)]
use bevy_ecs::{component::Component, query::With, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{BevyObject, DefaultInit, WorldExtension};
use serde_json::json;

#[derive(Component, TypePath, Default)]
pub struct A(char);

#[derive(BevyObject)]
pub struct B {
    a: DefaultInit<A>,
}

#[derive(BevyObject)]
pub struct C {
    #[serde(default)]
    a: DefaultInit<A>,
}


#[derive(BevyObject)]
pub struct D {
    #[serde(skip)]
    a: DefaultInit<A>,
}


#[test]
pub fn test() {
    let mut world = World::new();
    world.load::<C, _>(json!([{}])).unwrap();
    let mut query = world.query_filtered::<(), With<A>>();
    assert!(query.get_single(&world).is_ok());

    world.despawn_bound_objects::<C>();
    assert!(world.entities().is_empty());
    assert!(query.get_single(&world).is_err());

    world.load::<D, _>(json!([{}])).unwrap();
    let mut query = world.query_filtered::<(), With<A>>();
    assert!(query.get_single(&world).is_ok());

    world.despawn_bound_objects::<C>();
    assert!(world.load::<B, _>(json!([{}])).is_err());
    assert!(world.entities().is_empty());
    assert!(query.get_single(&world).is_err());

    world.load::<C, _>(json!([{"a": null}])).unwrap();
    assert!(query.get_single(&world).is_ok());

    let value = serde_json::to_value(&world.serialize_lens::<C>()).unwrap();

    assert_eq!(value, json!([{"a": null}]));

    let value = serde_json::to_value(&world.serialize_lens::<D>()).unwrap();

    assert_eq!(value, json!([{}]));

    world.despawn_bound_objects::<C>();
    world.load::<C, _>(json!([{"a": null}])).unwrap();
    assert!(query.get_single(&world).is_ok());

    world.despawn_bound_objects::<C>();
    world.load::<B, _>(json!([{"a": null}])).unwrap();
    assert!(query.get_single(&world).is_ok());
}
