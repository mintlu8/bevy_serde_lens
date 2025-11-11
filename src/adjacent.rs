use std::marker::PhantomData;

use bevy::ecs::query::{QueryData, ReleaseStateQueryData};
use bevy_serde_lens_core::{DeUtils, SerUtils};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{BevyObject, ZstInit};

type RItem<'t, T> = <<T as QueryData>::ReadOnly as QueryData>::Item<'t, 't>;

/// Serialize another [`QueryData`] on the same entity.
///
/// # Note
///
/// Components in this [`QueryData`] must be (de)serialized first.
pub trait SerializeAdjacent<A: QueryData>: QueryData {
    fn name() -> &'static str;

    fn serialize_adjacent<S: Serializer>(
        this: &<Self::ReadOnly as QueryData>::Item<'_, '_>,
        other: &<A::ReadOnly as QueryData>::Item<'_, '_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>;

    fn deserialize_adjacent<'de, D: Deserializer<'de>>(
        this: &<Self::ReadOnly as QueryData>::Item<'_, '_>,
        deserializer: D,
    ) -> Result<(), D::Error>;
}

/// Serialize another [`QueryData`] on the same entity, using [`SerializeAdjacent`].
///
/// # Note
///
/// Components in this [`QueryData`] must be (de)serialized first.
#[derive(Debug)]
pub struct Adjacent<A: SerializeAdjacent<B>, B: QueryData>(PhantomData<(A, B)>);

impl<A: SerializeAdjacent<B>, B: QueryData> ZstInit for Adjacent<A, B> {
    fn init() -> Self {
        Adjacent(PhantomData)
    }
}

impl<A: SerializeAdjacent<B>, B: QueryData> Serialize for Adjacent<A, B>
where
    A::ReadOnly: ReleaseStateQueryData,
    B::ReadOnly: ReleaseStateQueryData,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerUtils::with_query::<(A::ReadOnly, B::ReadOnly), S, _>(|(a, b)| {
            A::serialize_adjacent(&a, &b, serializer)
        })?
    }
}

impl<'de, A: SerializeAdjacent<B>, B: QueryData> Deserialize<'de> for Adjacent<A, B>
where
    A::ReadOnly: ReleaseStateQueryData,
    B::ReadOnly: ReleaseStateQueryData,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        DeUtils::with_query::<A::ReadOnly, D, _>(|a| A::deserialize_adjacent(&a, deserializer))??;
        Ok(ZstInit::init())
    }
}

struct AdjacentSerializeImpl<'t, A: SerializeAdjacent<B>, B: QueryData> {
    a: RItem<'t, A>,
    b: RItem<'t, B>,
}

impl<A: SerializeAdjacent<B>, B: QueryData> Serialize for AdjacentSerializeImpl<'_, A, B> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        A::serialize_adjacent(&self.a, &self.b, serializer)
    }
}

impl<A: SerializeAdjacent<B>, B: QueryData> BevyObject for Adjacent<A, B>
where
    A::ReadOnly: ReleaseStateQueryData,
    B::ReadOnly: ReleaseStateQueryData,
{
    type Object = Self;

    const IS_QUERY: bool = true;

    type Data = (A, B);

    type Filter = ();

    fn name() -> &'static str {
        A::name()
    }

    fn into_ser((a, b): crate::Item<'_, Self>) -> impl serde::Serialize {
        AdjacentSerializeImpl::<A, B> { a, b }
    }
}
