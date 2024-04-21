use bevy_ecs::{component::Component, query::With, system::Resource, world::World};
use bevy_serde_project::{batch, bind_object, BindResource, Named, SerdeProject, WorldExtension};
use serde_json::json;

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct A(char);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct B(f32);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct C(String);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct D(usize);

#[derive(SerdeProject, Resource)]
#[serde(transparent)]
pub struct R(usize);

bind_object!(A as "A");
bind_object!(B as "B");
bind_object!(C as "C");
bind_object!(D as "D");

bind_object!(pub struct ABWithCD = (With<A>, With<B>, With<C>, With<D>) as "AB" {
    a => A,
    b => B,
});

type AB = batch!(A, B);
type BD = batch!(B, D);
type CD = batch!(C, D);
type ABCD = batch!(A, B, C, D);

impl Named for R {
    fn name() -> &'static str {
        "R"
    }
}

type ABCDR = batch!(A, B, C, D, BindResource<R>);

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

    assert_eq!(value, json!([
        "b",
        "e",
        "v",
        "y"
    ]));

    let value = world.save::<B, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        3.0,
        0.5
    ]));

    let value = world.save::<C, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        "Ferris",
        "Crab"
    ]));

    let value = world.save::<D, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        69,
        420
    ]));

    let value = world.save::<AB, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!({
        "A": ["b", "e", "v", "y"],
        "B": [3.0, 0.5]
    }));


    let value = world.save::<BD, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!({
        "B": [3.0, 0.5],
        "D": [69, 420],
    }));

    let value = world.save::<ABCD, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!({
        "A": ["b", "e", "v", "y"],
        "B": [3.0, 0.5],
        "C": ["Ferris", "Crab"],
        "D": [69, 420],
    }));

    world.despawn_bound_objects::<AB>();
    assert_eq!(world.entities().len(), 4);

    world.despawn_bound_objects::<CD>();
    assert_eq!(world.entities().len(), 0);

    world.load::<ABCD, _>(&value).unwrap();
    assert_eq!(world.entities().len(), 10);

    let value = world.save::<ABCD, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!({
        "A": ["b", "e", "v", "y"],
        "B": [3.0, 0.5],
        "C": ["Ferris", "Crab"],
        "D": [69, 420],
    }));

    world.despawn_bound_objects::<ABCD>();
    assert_eq!(world.entities().len(), 0);

    world.load::<ABCD, _>(value).unwrap();

    world.insert_resource(R(12));

    let value = world.save::<ABCDR, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!({
        "A": ["b", "e", "v", "y"],
        "B": [3.0, 0.5],
        "C": ["Ferris", "Crab"],
        "D": [69, 420],
        "R": 12,
    }));

    world.despawn_bound_objects::<ABCDR>();
    
    assert_eq!(world.entities().len(), 0);

    assert!(!world.contains_resource::<R>());

    world.load::<ABCDR, _>(value).unwrap();

    assert_eq!(world.entities().len(), 10);

    assert!(world.contains_resource::<R>());

    world.despawn_bound_objects::<ABCDR>();

    world.spawn((
        A('y'),
        B(3.0),
        C("Ferris".to_owned()),
        D(69),
    ));

    world.spawn((
        A('z'),
        B(4.0),
    ));
    let value = world.save::<ABWithCD, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([
        {
            "a": "y",
            "b": 3.0,
        }
    ]));
}