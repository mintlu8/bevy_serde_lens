use bevy::pbr::{decal::ForwardDecal, ExtendedMaterial, Material, MaterialExtension, StandardMaterial};
use erased_serde::Serialize;
use serde::de::DeserializeOwned;

pub trait ExtendedStandardMaterial: Material {
    
}

impl ExtendedStandardMaterial for StandardMaterial {
    
}

impl<A: ExtendedStandardMaterial, E: MaterialExtension + Serialize + DeserializeOwned> ExtendedStandardMaterial for ExtendedMaterial<A, E> {
    
}

