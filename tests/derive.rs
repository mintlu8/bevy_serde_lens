use bevy::asset::{Asset, Handle};
use bevy::ecs::{bundle::Bundle, component::Component};
use bevy::reflect::TypePath;
use bevy_serde_lens::asset::{OwnedHandle, PathedHandle};
use bevy_serde_lens::{BevyObject, ChildVec, DefaultInit, Maybe};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, TypePath, Asset)]
pub struct Image;

#[derive(Debug, Serialize, Deserialize)]
struct MySprite(
    f32,
    f32,
    f32,
    f32,
    #[serde(with = "PathedHandle")] Handle<Image>,
    #[serde(with = "OwnedHandle")] Handle<Image>,
    PathedHandle<Image>,
    OwnedHandle<Image>,
);

#[derive(Debug, Component, Default)]
struct A;

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
