#![allow(clippy::upper_case_acronyms)]
use bevy_ecs::{component::Component, query::With, system::Resource, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{
    batch, bind_object, bind_query, ScopedDeserializeLens, SerializeResource, WorldExtension,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct A(char);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct B(f32);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct C(String);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct D(usize);

#[derive(Serialize, Deserialize, Resource, TypePath)]
#[serde(transparent)]
pub struct R(usize);

bind_object!(pub struct ABWithCD as (With<A>, With<B>, With<C>, With<D>) {
    a: A,
    b: B,
});

bind_query!(pub struct AQuery as A {
    a: A
});

bind_query!(pub struct AbcdQuery as (With<A>, With<B>, With<C>, With<D>) {
    a: A,
    b: B,
    c: C,
    d: D,
});

type AB = batch!(A, B);
type BD = batch!(B, D);
type CD = batch!(C, D);
type ABCD = batch!(A, B, C, D);

type ABCDR = batch!(A, B, C, D, SerializeResource<R>);

#[test]
pub fn test() {
    let mut world = World::new();
    world.spawn(A('b'));
    world.spawn(A('e'));
    world.spawn(A('v'));
    world.spawn(A('y'));
    world.spawn(B(3.0));
    world.spawn(B(0.5));
    world.spawn(C("Ferris".to_owned()));
    world.spawn(C("Crab".to_owned()));
    world.spawn(D(69));
    world.spawn(D(420));

    let value = world.save::<A, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!(["b", "e", "v", "y"]));

    let value = world.save::<B, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([3.0, 0.5]));

    let value = world.save::<C, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!(["Ferris", "Crab"]));

    let value = world.save::<D, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([69, 420]));

    let value = world.save::<AB, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(
        value,
        json!({
            "A": ["b", "e", "v", "y"],
            "B": [3.0, 0.5]
        })
    );

    let value = world.save::<BD, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(
        value,
        json!({
            "B": [3.0, 0.5],
            "D": [69, 420],
        })
    );

    let value = world
        .save::<ABCD, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(
        value,
        json!({
            "A": ["b", "e", "v", "y"],
            "B": [3.0, 0.5],
            "C": ["Ferris", "Crab"],
            "D": [69, 420],
        })
    );

    world.despawn_bound_objects::<AB>();
    assert_eq!(world.entities().len(), 4);

    world.despawn_bound_objects::<CD>();
    assert_eq!(world.entities().len(), 0);

    world.load::<ABCD, _>(&value).unwrap();

    assert_eq!(world.entities().len(), 10);

    let value = world
        .save::<ABCD, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(
        value,
        json!({
            "A": ["b", "e", "v", "y"],
            "B": [3.0, 0.5],
            "C": ["Ferris", "Crab"],
            "D": [69, 420],
        })
    );

    world.despawn_bound_objects::<ABCD>();
    assert_eq!(world.entities().len(), 0);

    world.load::<ABCD, _>(value).unwrap();

    world.insert_resource(R(12));

    let lens = world.serialize_lens::<ABCDR>();

    let value = serde_json::to_value(lens).unwrap();

    assert_eq!(
        value,
        json!({
            "A": ["b", "e", "v", "y"],
            "B": [3.0, 0.5],
            "C": ["Ferris", "Crab"],
            "D": [69, 420],
            "R": 12,
        })
    );

    world.despawn_bound_objects::<ABCDR>();

    assert_eq!(world.entities().len(), 0);

    assert!(!world.contains_resource::<R>());

    world.load::<ABCDR, _>(value.clone()).unwrap();

    assert_eq!(world.entities().len(), 10);

    assert!(world.contains_resource::<R>());

    world.despawn_bound_objects::<ABCDR>();

    world.scoped_deserialize_lens(|| {
        let _: ScopedDeserializeLens<ABCDR> = serde_json::from_value(value).unwrap();
    });

    assert_eq!(world.entities().len(), 10);

    assert!(world.contains_resource::<R>());

    world.despawn_bound_objects::<ABCDR>();

    world.spawn((A('y'), B(3.0), C("Ferris".to_owned()), D(69)));

    world.spawn((A('z'), B(4.0)));
    let value = world
        .save::<ABWithCD, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(
        value,
        json!([
            {
                "a": "y",
                "b": 3.0,
            }
        ])
    );
    world.despawn_bound_objects::<ABWithCD>();
    assert!(!world.entities().is_empty());
    world.clear_all();

    world.spawn((A('y'), B(3.0), C("Ferris".to_owned()), D(69)));

    world.spawn((A('z'), B(4.5), C("Gopher".to_owned()), D(32)));

    let value = world
        .save::<AbcdQuery, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(
        value,
        json!([
            {
                "a": "y",
                "b": 3.0,
                "c": "Ferris",
                "d": 69
            },
            {
                "a": "z",
                "b": 4.5,
                "c": "Gopher",
                "d": 32
            },
        ])
    );
}
