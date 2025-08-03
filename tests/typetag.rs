use bevy::ecs::component::{Mutable, StorageType};
use bevy::ecs::{component::Component, world::World};
use bevy::reflect::DynamicTypePath;
use bevy::reflect::TypePath;
use bevy_serde_lens::typetagged::ErasedObject;
use bevy_serde_lens::typetagged::TypeTagged;
use bevy_serde_lens::{BevyObject, Maybe, WorldExtension};
use postcard::ser_flavors::Flavor;
use serde::{Deserialize, Serialize};
use serde_json::json;

macro_rules! impl_animal {
    ($($ty: ident),*) => {
        $(impl Animal for $ty {
            fn as_ser(&self) -> &dyn erased_serde::Serialize {
                self
            }
        })*
    };
}
macro_rules! boxed_animal {
    ($expr: expr) => {{
        let val: Box<dyn Animal> = Box::new($expr);
        val
    }};
}
pub trait Animal: DynamicTypePath + Send + Sync + 'static {
    fn as_ser(&self) -> &dyn erased_serde::Serialize;
}

impl<T> From<T> for Box<dyn Animal>
where
    T: Animal + 'static,
{
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl ErasedObject for Box<dyn Animal> {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().reflect_short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ser()
    }
}

#[derive(Debug, Serialize, Deserialize, TypePath)]
pub struct Bird(String);

#[derive(Debug, Serialize, Deserialize, TypePath)]
pub struct Dog {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, TypePath)]
pub struct Turtle;

impl_animal!(Bird, Dog, Turtle);

#[derive(Component, Serialize, Deserialize, TypePath)]
pub struct AnimalComponent {
    #[serde(with = "TypeTagged")]
    animal: Box<dyn Animal>,
}

#[test]
pub fn test() {
    let mut world = World::new();
    world.register_typetag::<Box<dyn Animal>, Bird>();
    world.register_typetag::<Box<dyn Animal>, Dog>();
    world.register_typetag::<Box<dyn Animal>, Turtle>();
    world.spawn(AnimalComponent {
        animal: boxed_animal!(Dog {
            name: "Rex".to_owned()
        }),
    });
    let value = world
        .save::<AnimalComponent, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(value, json!([{"animal": {"Dog": {"name": "Rex"}}}]));
    world.spawn(AnimalComponent {
        animal: boxed_animal!(Bird("bevy".to_owned())),
    });
    let value = world
        .save::<AnimalComponent, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(
        value,
        json!([
            {"animal": {"Dog": {"name": "Rex"}}},
            {"animal": {"Bird": "bevy"}}
        ])
    );
    world.spawn(AnimalComponent {
        animal: boxed_animal!(Turtle),
    });
    let value = world
        .save::<AnimalComponent, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(
        value,
        json!([
            {"animal": {"Dog": {"name": "Rex"}}},
            {"animal": {"Bird": "bevy"}},
            {"animal": {"Turtle": null}},
        ])
    );

    let value = world
        .save::<AnimalComponent, _>(serde_json::value::Serializer)
        .unwrap();

    world.despawn_bound_objects::<AnimalComponent>();
    assert_eq!(world.entities().len(), 0);
    world.load::<AnimalComponent, _>(&value).unwrap();
    assert_eq!(world.entities().len(), 3);

    let value = world
        .save::<AnimalComponent, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(
        value,
        json!([
            {"animal": {"Dog": {"name": "Rex"}}},
            {"animal": {"Bird": "bevy"}},
            {"animal": {"Turtle": null}},
        ])
    );

    let mut vec = postcard::Serializer {
        output: postcard::ser_flavors::AllocVec::new(),
    };
    world.save::<AnimalComponent, _>(&mut vec).unwrap();
    let result = vec.output.finalize().unwrap();

    world.despawn_bound_objects::<AnimalComponent>();
    assert_eq!(world.entities().len(), 0);

    let mut de = postcard::Deserializer::from_bytes(&result);
    world.load::<AnimalComponent, _>(&mut de).unwrap();
    assert_eq!(world.entities().len(), 3);

    let value = world
        .save::<AnimalComponent, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(
        value,
        json!([
            {"animal": {"Dog": {"name": "Rex"}}},
            {"animal": {"Bird": "bevy"}},
            {"animal": {"Turtle": null}},
        ])
    );
}

impl TypePath for Box<dyn Animal> {
    fn type_path() -> &'static str {
        "Animal"
    }

    fn short_type_path() -> &'static str {
        "Animal"
    }
}

impl Component for Box<dyn Animal> {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    type Mutability = Mutable;
}

#[derive(BevyObject)]
#[serde(transparent)]
pub struct SerializeAnimal {
    pub animal: TypeTagged<Box<dyn Animal>>,
}

#[derive(BevyObject)]
pub struct SerializeAnimalMaybe {
    pub animal: Maybe<TypeTagged<Box<dyn Animal>>>,
}
#[test]
pub fn test2() {
    let mut world = World::new();
    world.spawn(Box::new(Dog {
        name: "Loki".into(),
    }) as Box<dyn Animal>);
    world.spawn(Box::new(Dog {
        name: "Bella".into(),
    }) as Box<dyn Animal>);
    world.register_typetag::<Box<dyn Animal>, Dog>();

    let value = world
        .save::<SerializeAnimal, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(
        value,
        json!([
            {
                "Dog": {
                    "name": "Loki"
                }
            },
            {
                "Dog": {
                    "name": "Bella"
                }
            },
        ])
    );

    world.despawn_bound_objects::<SerializeAnimal>();

    world.load::<SerializeAnimal, _>(&value).unwrap();

    assert_eq!(world.entities().len(), 2);

    let value2 = world
        .save::<SerializeAnimal, _>(serde_json::value::Serializer)
        .unwrap();

    assert_eq!(value, value2);
}
