use crate::{BevyObject, SerializeComponent, ZstInit};
use bevy::ecs::{component::Component, query::With};
use bevy::reflect::TypePath;
use bevy_serde_lens_core::{DeUtils, SerUtils};
use ref_cast::RefCast;
use serde::Deserialize;
use serde::Serialize;

use super::{AnyOrTagged, ErasedObject, SmartTagged, TypeTagged};

macro_rules! impl_for {
    ($($ty: ident),*) => {
$(

impl<T: Component + ErasedObject> ZstInit for $ty<SerializeComponent<T>> {
    fn init() -> Self {
        $ty(SerializeComponent::init())
    }
}

impl<T: Component + TypePath + ErasedObject> BevyObject for $ty<T> {
    type Object = $ty<SerializeComponent<T>>;

    const IS_QUERY: bool = true;

    type Data = &'static T;

    type Filter = With<T>;

    fn name() -> &'static str {
        T::short_type_path()
    }

    fn into_ser(query_data: $crate::Item<'_, Self>) -> impl Serialize {
        $ty::ref_cast(query_data)
    }
}

impl<T: Component + ErasedObject> serde::Serialize for $ty<SerializeComponent<T>>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerUtils::with_component::<T, S, _>(|c| {
            $ty::ref_cast(c).serialize(serializer)
        })?
    }
}

impl<'de, T: Component + ErasedObject> Deserialize<'de> for $ty<SerializeComponent<T>> {
    fn deserialize<D: serde::Deserializer<'de>,>(deserializer: D) -> Result<Self, D::Error>
    {
        let component = $ty::<T>::deserialize(deserializer)?;
        DeUtils::insert::<D>(component)?;
        Ok(ZstInit::init())
    }
}
)*
    };
}

impl_for!(TypeTagged, AnyOrTagged, SmartTagged);
