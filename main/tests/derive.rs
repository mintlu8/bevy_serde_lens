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

#[derive(Debug, SerdeProject)]
struct MySprite2 (
    f32,
    f32,
    f32,
    f32,
    #[serde_project(ignore)]
    WeirdCacheThing,
    #[serde_project("PathHandle<Image>")]
    Handle<Image>
);

#[derive(Debug, SerdeProject)]
enum Never {}


#[derive(Debug, SerdeProject)]
enum MyImage {
    Handle(#[serde_project("PathHandle<Image>")] Handle<Image>)
}


#[test]
pub fn test() {
    type S<'t> = <MySprite as SerdeProject>::Ser<'t>;
}