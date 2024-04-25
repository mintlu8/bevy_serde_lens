use bevy_asset::{Asset, Handle};
use bevy_reflect::TypePath;
use bevy_serde_lens::asset::{PathHandle, UniqueHandle};
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


#[test]
pub fn test() {}