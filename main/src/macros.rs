#[allow(unused)]
use crate::{BevyObject, Component, BindBevyObject, Object, Maybe, SerdeProject};

#[doc(hidden)]
#[macro_export]
macro_rules! parse_name {
    ($orig: ty) => { ::std::any::type_name::<$orig>() };
    ($orig: ty as $lit: literal) => { $lit };
    ($orig: ty as $ident: ident) => { stringify!($ident) };
}

/// Bind a [`BevyObject`] to a [`Component`].
///
/// The type is unnameable but can be accessed via [`BindBevyObject::BevyObject`]
/// or the [`Object`] extractor.
///
/// # Syntax
///
/// ```
/// // `as "string"`` or `as ident` sets the serialized name,
/// // if not set this is `std::any::type_name()`.
/// bind_object!(Weapon as "weapon" {
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
///     enchants => ChildList<Enchant>,
///     // Find and serialize all `BevyObject`s `Gem` in children like a `Vec`.
///     // Note without `Object` we would serialize components `Gem` instead.
///     gems => ChildList<Object<Gem>>,
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
/// #[serde(default, skip_deserializing_if = "Option::None")]
/// ```
/// can be used to skip a [`Maybe`] field if None, but this will
/// break non-self-describing formats.
#[macro_export]
macro_rules! bind_object {
    ($(#[$($head_attr: tt)*])* $main: ty $(as $name: tt)? {
        $($(#[$($attr: tt)*])* $field: ident => $ty: ty),* $(,)?
    }) => {
        #[allow(unused)]
        const _: () = {
            impl $crate::BindBevyObject for $main {
                type BevyObject = __BoundObject;

                fn name() -> &'static str {
                    $crate::parse_name!($main $(as $name)?)
                }
            }

            pub struct __BoundObject;

            #[derive($crate::serde::Serialize)]
            $(#[$($head_attr)*])*
            pub struct __Ser<'t> {
                $(
                    $(#[$($attr)*])*
                    $field: <$ty as $crate::BevyObject>::Ser<'t>,
                )*
                #[serde(skip)]
                __phantom: ::std::marker::PhantomData<&'t ()>
            }

            #[derive($crate::serde::Deserialize)]
            #[serde(bound = "'t: 'de")]
            $(#[$($head_attr)*])*
            pub struct __De<'t> {
                $(
                    $(#[$($attr)*])*
                    $field: <$ty as $crate::BevyObject>::De<'t>,
                )*
                #[serde(skip)]
                __phantom: ::std::marker::PhantomData<&'t ()>
            }
    
            impl $crate::BevyObject for __BoundObject {
                type Ser<'t> = __Ser<'t>;
                type De<'de> = __De<'de>;
                fn to_ser(world: & $crate::World, entity: $crate::Entity) -> Result<Option<Self::Ser<'_>>, Box<$crate::Error>> {
                    Ok(Some(__Ser {
                        $($field: <$ty as $crate::BevyObject>::to_ser(world, entity)?
                            .ok_or_else(||$crate::Error::FieldMissing {
                                field: stringify!($field),
                                ty: <$main as $crate::BindBevyObject>::name()
                            }.boxed())?,
                        )*
                        __phantom: ::std::marker::PhantomData,
                    }))
                }
    
                fn from_de(world: &mut $crate::World, parent: $crate::Entity, de: Self::De<'_>) -> Result<(), Box<$crate::Error>> {
                    $(<$ty as $crate::BevyObject>::from_de(world, parent, de.$field)?;)*
                    Ok(())
                }
            };
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
    ($ty: ty) => {
        $ty
    };
    ($a: ty, $b: ty $(,)?) => {
        $crate::Join<$a, $b>
    };
    ($first: ty $(,$ty: ty)* $(,)?) => {
        $crate::Join<$a, $crate::join_types!($($ty)*)>
    };
}

#[derive(Debug, Clone, Copy, crate::Component, crate::Serialize, crate::Deserialize)]
struct A;

bind_object!(A {
    this => A,
    #[serde(flatten)]
    other => crate::Child<A>,
});
