use bevy_asset::{Asset, Handle};
use bevy_ecs::{component::Component, query::With};
use bevy_reflect::TypePath;
use bevy_serde_lens::{asset::{PathHandle, UniqueHandle}, bind_object, DefaultInit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, TypePath, Asset)]
pub struct Image;

#[derive(Debug, Serialize, Deserialize)]
struct MySprite (
    f32,
    f32,
    f32,
    f32,
    #[serde(with = "PathHandle")]
    Handle<Image>,
    #[serde(with = "UniqueHandle")]
    Handle<Image>,
    PathHandle<Image>,
    UniqueHandle<Image>,
);

#[derive(Debug, Component, Default)]
struct A;

bind_object!(struct B as (With<A>) {
    #[serde(default)]
    a: DefaultInit<A>,
});

#[test]
pub fn test() {}