use bevy::{asset::Handle, image::Image};
use serde::Serialize;
use wgpu_types::Extent3d;

use crate::MappedSerializer;

pub struct SerializeImage;

#[derive(Serialize)]
pub enum SerImageInner<'t> {
    U8(&'t [u8]),
    F32(&'t [f32]),
}

#[derive(Serialize)]
pub struct SerImage<'t> {
    pub size: Extent3d,
    pub data: SerImageInner<'t>,
}

impl MappedSerializer<Image> for SerializeImage {
    fn serialize<S: serde::Serializer>(image: &Image, serializer: S) -> Result<S::Ok, S::Error> {
        SerImage
    }

    fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Image, D::Error> {
        todo!()
    }
}