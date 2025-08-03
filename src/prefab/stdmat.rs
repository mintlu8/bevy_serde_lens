use bevy::{asset::Handle, color::{Color, LinearRgba}, image::Image, math::Affine2, pbr::{
    ExtendedMaterial, Material, MaterialExtension, OpaqueRendererMethod, ParallaxMappingMethod, StandardMaterial, UvChannel
}, render::alpha::AlphaMode};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use wgpu_types::Face;

use crate::MappedSerializer;

#[derive(Debug, Serialize)]
pub struct SerExMat<'t, B, E>(&'t B, &'t E);


#[derive(Serialize, Deserialize)]
pub enum UvChannel2 {
    Uv0, Uv1,
}

#[derive(Serialize)]
pub struct SerStandardMaterial {
    pub base_color: Color,
    pub base_color_channel: UvChannel2,
    pub base_color_texture: Option<Handle<Image>>,
    pub emissive: LinearRgba,
    pub emissive_exposure_weight: f32,
    pub emissive_channel: UvChannel2,
    pub emissive_texture: Option<Handle<Image>>,
    pub perceptual_roughness: f32,
    pub metallic: f32,
    pub metallic_roughness_channel: UvChannel2,
    pub metallic_roughness_texture: Option<Handle<Image>>,
    pub reflectance: f32,
    pub specular_tint: Color,
    pub diffuse_transmission: f32,
    pub diffuse_transmission_channel: UvChannel2,
    pub diffuse_transmission_texture: Option<Handle<Image>>,
    pub specular_transmission: f32,
    pub specular_transmission_channel: UvChannel2,
    pub specular_transmission_texture: Option<Handle<Image>>,
    pub thickness: f32,
    pub thickness_channel: UvChannel2,
    pub thickness_texture: Option<Handle<Image>>,
    pub ior: f32,
    pub attenuation_distance: f32,
    pub attenuation_color: Color,
    pub normal_map_channel: UvChannel2,
    pub normal_map_texture: Option<Handle<Image>>,
    pub flip_normal_map_y: bool,
    pub occlusion_channel: UvChannel2,
    pub occlusion_texture: Option<Handle<Image>>,
    pub specular_channel: UvChannel2,
    pub specular_texture: Option<Handle<Image>>,
    pub specular_tint_channel: UvChannel2,
    pub specular_tint_texture: Option<Handle<Image>>,
    pub clearcoat: f32,
    pub clearcoat_channel: UvChannel2,
    pub clearcoat_texture: Option<Handle<Image>>,
    pub clearcoat_perceptual_roughness: f32,
    pub clearcoat_roughness_channel: UvChannel2,
    pub clearcoat_roughness_texture: Option<Handle<Image>>,
    pub clearcoat_normal_channel: UvChannel2,
    pub clearcoat_normal_texture: Option<Handle<Image>>,
    pub anisotropy_strength: f32,
    pub anisotropy_rotation: f32,
    pub anisotropy_channel: UvChannel2,
    pub anisotropy_texture: Option<Handle<Image>>,
    pub double_sided: bool,
    pub cull_mode: Option<Face>,
    pub unlit: bool,
    pub fog_enabled: bool,
    pub alpha_mode: AlphaMode,
    pub depth_bias: f32,
    pub depth_map: Option<Handle<Image>>,
    pub parallax_depth_scale: f32,
    pub parallax_mapping_method: ParallaxMappingMethod,
    pub max_parallax_layer_count: f32,
    pub lightmap_exposure: f32,
    pub opaque_render_method: OpaqueRendererMethod,
    pub deferred_lighting_pass_id: u8,
    pub uv_transform: Affine2,
}

pub struct SerExStdMat;

impl<T> MappedSerializer<T> for SerExStdMat {
    fn serialize<S: Serializer>(item: &T, serializer: S) -> Result<S::Ok, S::Error> {
        todo!()
    }

    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        todo!()
    }
}