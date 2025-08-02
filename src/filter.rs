use bevy::ecs::query::{Or, QueryFilter, With, Without};
use bevy::ecs::{component::Component, world::EntityRef};

/// A subset of [`QueryFilter`] that works on [`EntityRef`].
/// Supports tuples, [`With`], [`Without`] and [`Or`].
pub trait EntityFilter: QueryFilter {
    fn filter(entity: &EntityRef) -> bool;
}

impl EntityFilter for () {
    fn filter(_: &EntityRef) -> bool {
        true
    }
}

impl EntityFilter for Or<()> {
    fn filter(_: &EntityRef) -> bool {
        true
    }
}

impl<T> EntityFilter for With<T>
where
    T: Component,
{
    fn filter(entity: &EntityRef) -> bool {
        entity.contains::<T>()
    }
}

impl<T> EntityFilter for Without<T>
where
    T: Component,
{
    fn filter(entity: &EntityRef) -> bool {
        !entity.contains::<T>()
    }
}

macro_rules! impl_tuple {
    () => {};
    ($f: ident $(, $n: ident)*) => {
        impl_tuple!($($n),*);
        impl<$f, $($n),*> EntityFilter for ($f, $($n,)*) where $f: EntityFilter, $($n: EntityFilter),* {
            fn filter(entity: &EntityRef) -> bool {
                $f::filter(entity) $(&& $n::filter(entity))*
            }
        }

        impl<$f, $($n),*> EntityFilter for Or<($f, $($n,)*)> where $f: EntityFilter, $($n: EntityFilter),* {
            fn filter(entity: &EntityRef) -> bool {
                $f::filter(entity) $(|| $n::filter(entity))*
            }
        }
    };
}

impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
