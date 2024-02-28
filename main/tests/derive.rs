use bevy_asset::{Asset, Handle};
use bevy_reflect::TypePath;
use bevy_serde_project::SerdeProject;
use bevy_serde_project::asset::PathHandle;

#[derive(Debug, Clone, TypePath, Asset)]
pub struct Image(String);

#[derive(Debug, Default)]
pub struct WeirdCacheThing(Vec<u8>);

#[derive(Debug, SerdeProject)]
struct MySprite {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde_project(ignore)]
    pub cache: WeirdCacheThing,
    #[serde_project("PathHandle<Image>")]
    pub handle: Handle<Image>
}

#[test]
pub fn test() {
    type S<'t> = <MySprite as SerdeProject>::Ser<'t>;
}