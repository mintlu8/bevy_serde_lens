use bevy_asset::{Asset, Handle};
use bevy_reflect::TypePath;
use bevy_serde_project::SerdeProject;
use bevy_serde_project::asset::PathHandle;

// This is not allowed
// #[derive(SerdeProject)]
// pub struct Nil;

// This just tests the derive macro.
#[derive(SerdeProject)]
pub struct Nil();

#[derive(SerdeProject)]
pub struct Nil2{}

#[derive(SerdeProject)]
pub enum Never{}

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
enum MyImage {
    Handle(#[serde_project("PathHandle<Image>")] Handle<Image>)
}

#[derive(Debug, SerdeProject)]
enum MyImage2 {
    None,
    Indexes(i32, u32, f32),
    Handle(#[serde_project("PathHandle<Image>")] Handle<Image>),
    Handles{
        #[serde_project("PathHandle<Image>")] image1: Handle<Image>,
        #[serde_project("PathHandle<Image>")] image2: Handle<Image>,
    }
}


#[test]
pub fn test() {
    type S<'t> = <MySprite as SerdeProject>::Ser<'t>;
}