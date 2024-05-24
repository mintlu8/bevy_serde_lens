use bevy_asset::{Asset, Handle};
use bevy_ecs::{component::Component, query::With};
use bevy_reflect::TypePath;
use bevy_serde_lens::{
    asset::{PathHandle, UniqueHandle},
    bind_object, bind_query, DefaultInit,
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

#[derive(Debug, Component, Serialize, Deserialize, TypePath)]
struct Ddd;

bind_object!(
    struct X {
        a: Aaa,
        b: Bbb,
        c: Ccc,
        d: Ddd,
    }
);

bind_query!(
    struct Y {
        a: Aaa,
        b: Bbb,
        c: Ccc,
        d: Ddd,
    }
);

#[test]
pub fn test() {}
