use bevy::prelude::{GlobalTransform, InheritedVisibility, Transform, ViewVisibility, Visibility};
use bevy_serde_lens::{AdaptedComponent, BevyObject, DefaultInit, SerdeAdaptor, SerializeComponent};

pub struct VisibilityAdapt;

impl SerdeAdaptor for VisibilityAdapt {
    type Type = Visibility;

    fn serialize<S: serde::Serializer>(serializer: S, item: &Self::Type) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            match item {
                Visibility::Inherited => serializer.serialize_char('I'),
                Visibility::Hidden => serializer.serialize_char('H'),
                Visibility::Visible => serializer.serialize_char('V'),
            }
        } else {
            match item {
                Visibility::Inherited => serializer.serialize_u8(b'I'),
                Visibility::Hidden => serializer.serialize_u8(b'H'),
                Visibility::Visible => serializer.serialize_u8(b'V'),
            }
        }
    }

    fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self::Type, D::Error> {
        use serde::Deserialize;
        if deserializer.is_human_readable() {
            let c = char::deserialize(deserializer)?;
            Ok(match c {
                'I' => Visibility::Inherited,
                'H' => Visibility::Hidden,
                'V' => Visibility::Visible,
                _ => return Err(serde::de::Error::custom("Expected 'I', 'H' or 'V'."))
            })
        } else {
            let b = u8::deserialize(deserializer)?;
            Ok(match b {
                b'I' => Visibility::Inherited,
                b'H' => Visibility::Hidden,
                b'V' => Visibility::Visible,
                _ => return Err(serde::de::Error::custom("Expected 'I', 'H' or 'V'."))
            })
        }
    }
}

#[derive(BevyObject)]
pub struct GltfNode {
    pub transform: Transform,
    #[serde(with = "AdaptedComponent::<VisibilityAdapt>")]
    pub visibility: SerializeComponent<Visibility>,
    #[serde(skip)]
    pub global: DefaultInit<GlobalTransform>,
    #[serde(skip)]
    pub view_visibility: DefaultInit<ViewVisibility>,
    #[serde(skip)]
    pub inherited: DefaultInit<InheritedVisibility>,
}