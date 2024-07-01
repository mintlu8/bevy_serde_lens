use bevy_asset::{Asset, Handle};
use bevy_ecs::{bundle::Bundle, component::Component, query::With};
use bevy_reflect::TypePath;
use bevy_serde_lens::{
    asset::{PathHandle, UniqueHandle},
    bind_object, BevyObject, ChildVec, DefaultInit, Maybe,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, TypePath, Asset)]
pub struct Image;

#[derive(Debug, Serialize, Deserialize)]
struct MySprite(
    f32,
    f32,
    f32,
    f32,
    #[serde(with = "PathHandle")] Handle<Image>,
    #[serde(with = "UniqueHandle")] Handle<Image>,
    PathHandle<Image>,
    UniqueHandle<Image>,
);

#[derive(Debug, Component, Default)]
struct A;

bind_object!(struct B as (With<A>) {
    #[serde(default)]
    a: DefaultInit<A>,
});

#[derive(Debug, Component, Serialize, Deserialize, TypePath)]
struct Aaa;

#[derive(Debug, Component, Serialize, Deserialize, TypePath)]
struct Bbb;

#[derive(Debug, Component, Serialize, Deserialize, TypePath)]
struct Ccc;

#[derive(Debug, Default, Component, Serialize, Deserialize, TypePath)]
struct Ddd;

#[derive(Debug, Bundle, BevyObject)]
struct Xa {
    a: Aaa,
    b: Bbb,
    c: Ccc,
    d: Ddd,
}

#[derive(Debug, Bundle, BevyObject)]
#[bevy_object(query, rename = "xb")]
struct Xb {
    a: Aaa,
    b: Bbb,
    c: Ccc,
    #[bevy_object(no_filter)]
    d: Ddd,
}

#[derive(Debug, BevyObject)]
#[bevy_object(rename = "xb")]
struct Xc {
    #[bevy_object(no_filter)]
    a: Aaa,
    #[serde(default)]
    b: Maybe<Bbb>,
    c: ChildVec<Ccc>,
    d: DefaultInit<Ddd>,
}

#[test]
pub fn test() {}
