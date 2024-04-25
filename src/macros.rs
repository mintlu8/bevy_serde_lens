#[allow(unused)]
use crate::{Component, BevyObject, Maybe, TypePath};
#[allow(unused)]
use bevy_ecs::query::QueryFilter;
/// Bind a [`BevyObject`] to a [`QueryFilter`].
///
/// # Syntax
///
/// ```
/// // Bind a `QueryFilter` to a `BevyObject`
/// // The generated object must satisfy the `QueryFilter` to roundtrip properly.
/// bind_object!(SerializeWeapon as (With<Weapon>, Without<Unusable>) {
///     // `serde` attributes are allowed.
///     #[serde(flatten)]
///     // Serialize the main component, this is required.
///     this => Weapon,
///     // Find and serialize component `Durability`, error if not found.
///     durability => Durability,
///     // Find and serialize component `CustomName` as an `Option<CustomName>`.
///     // Without Maybe not finding `CustomName` would be an error.
///     custom_name => Maybe<CustomName>,
///     // Find and serialize all components `Enchant` in children like a `Vec`.
///     #[serde(default, skip_serializing_if = "Vec::None")]
///     enchants => ChildVec<Enchant>,
///     // Find and serialize all `BevyObject`s `Gem` in children like a `Vec`.
///     // Note without `Object` we would serialize components `Gem` instead.
///     gems => ChildVec<Object<Gem>>,
///     // Find zero or one component `Forge` in children as an `Option<Forge>`.
///     // Errors if more than one found.
///     forge => Child<Maybe<Forge>>,
/// });
/// ```
///
/// # Note
///
/// You can specify serde attributes on fields.
/// In order for the structs to roundtrip properly,
/// you must use the correct serde attributes.
/// This can be a bit footgun heavy so reading the serde
/// documentation is recommended.
///
/// For example 
/// ```
/// #[serde(default)]
/// ```
/// can be used to skip a [`Maybe`] field if None. 
/// Keep in mind this will break non-self-describing formats.
/// 
/// # Rename
/// 
/// The type derives [`TypePath`] and uses `short_type_path` as its name,
/// use `type_path` attributes to rename the type if desired.
#[macro_export]
macro_rules! bind_object {
    ($(#[$($head_attr: tt)*])* $vis: vis struct $main: ident as $filter: ident {$($tt:tt)*}) => {
        $crate::bind_object!(
            $(#[$($head_attr)*])* $vis struct $main as $crate::With<$filter> {$($tt)*}
        );
    };

    ($(#[$($head_attr: tt)*])* $vis: vis struct $main: ident as $filter: ty ) => {
        #[allow(unused)]
        const _: () = {
            impl $crate::BevyObject for $main {
                type Filter = $filter;
                type Object = $main;

                fn name() -> &'static str {
                    $name
                }
            }
        };
    };

    ($(#[$($head_attr: tt)*])* $vis: vis struct $main: ident as $filter: ty  {
        $($(#[$($attr: tt)*])* $field: ident: $ty: ty),* $(,)?
    }) => {

        #[derive($crate::serde::Serialize, $crate::serde::Deserialize, $crate::TypePath)]
        pub struct $main {
            $(
                $(#[$($attr)*])*
                $field: <$ty as $crate::BindProject>::To,
            )*
        }

        #[allow(unused)]
        const _: () = {
            impl $crate::BevyObject for $main {
                type Filter = $filter;
                type Object = $main;

                fn name() -> &'static str {
                    Self::short_type_path()
                }
            }

            impl $crate::ZstInit for $main {
                fn init() -> Self {
                    Self {
                        $($field: $crate::ZstInit::init(),)*
                    }
                }
            }
        };
    }
}

/// Batches multiple [`BindBevyObject`] types to be serialized together as a map.
///
/// This macro generates a `type` that can be used on `World::save` and `World::load`.
///
/// # Example
///
/// ```
/// type SerializeItems = serialize_group!(Potion, Weapon, Armor);
/// ```
#[macro_export]
macro_rules! batch {
    ($vis: vis type $ty: ident = ($($tt:tt)*)) => {
        mod paste::paste![<__sealed_ $ty>]{
            use $crate::Root;
            use $crate::SerializeResource as Res;
            $vis type $ty = $crate::batch_inner!($($tt)*);
        }
        $vis type $ty = __sealed::$ty;
    };
    ($($tt:tt)*) => {
        $crate::batch_inner!($($tt)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! batch_inner {
    ($ty: ty $(,)?) => {
        $ty
    };
    ($a: ty, $b: ty $(,)?) => {
        $crate::Join<$a, $b>
    };
    ($first: ty $(,$ty: ty)* $(,)?) => {
        $crate::Join<$first, $crate::batch_inner!($($ty),*)>
    };
}
