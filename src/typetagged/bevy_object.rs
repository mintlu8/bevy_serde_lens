use crate::{world_entity_scope, world_entity_scope_mut, BevyObject, SerializeComponent, ZstInit};
use bevy_ecs::{component::Component, query::With};
use bevy_reflect::TypePath;
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
        world_entity_scope::<_, S>(|world, entity| {
            let Ok(entity) = world.get_entity(entity) else {
                return Err(serde::ser::Error::custom(format!(
                    "Entity missing: {entity:?}."
                )));
            };
            let Some(component) = entity.get::<T>() else {
                return Err(serde::ser::Error::custom(format!(
                    "Component missing: {}.",
                    std::any::type_name::<T>()
                )));
            };
            $ty::ref_cast(component).serialize(serializer)
        })?
    }
}

impl<'de, T: Component + ErasedObject> Deserialize<'de> for $ty<SerializeComponent<T>> {
    fn deserialize<D: serde::Deserializer<'de>,>(deserializer: D) -> Result<Self, D::Error>
    {
        let component = $ty::<T>::deserialize(deserializer)?;
        world_entity_scope_mut::<_, D>(|world, entity| {
            let Ok(mut entity) = world.get_entity_mut(entity) else {
                return Err(serde::de::Error::custom(format!(
                    "Entity missing {entity:?}."
                )));
            };
            entity.insert(component);
            Ok(Self::init())
        })?
    }
}
)*
    };
}

impl_for!(TypeTagged, AnyOrTagged, SmartTagged);
