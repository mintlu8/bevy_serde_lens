use bevy_ecs::{component::Component, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{BevyObject, ChildVec, Maybe, WorldExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct Unit(String);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct Weapon(String);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct Armor(String);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct Potion(String);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct Ability(String);

#[derive(Serialize, Deserialize, Component, TypePath)]
#[serde(transparent)]
pub struct Effect(String);

#[derive(BevyObject)]
pub struct SerializeUnit {
    unit: Unit,
    #[serde(default)]
    #[bevy_object(no_filter)]
    weapon: Maybe<Weapon>,
    #[serde(default)]
    #[bevy_object(no_filter)]
    armor: Maybe<Armor>,
    #[serde(default)]
    potions: ChildVec<Potion>,
    #[serde(default)]
    abilities: ChildVec<SerializeAbility>,
}

#[derive(BevyObject)]

pub struct SerializeAbility {
    ability: Ability,
    effects: ChildVec<Effect>,
}

#[test]
pub fn test() {
    let mut world = World::new();
    world.spawn(Unit("Bob".to_owned()));
    world
        .spawn((Unit("Eric".to_owned()), Weapon("Sword".to_owned())))
        .with_children(|b| {
            b.spawn(Potion("Hp Potion".to_owned()));
            b.spawn(Potion("Mp Potion".to_owned()));
            b.spawn(Ability("Thrust".to_owned())).with_children(|b| {
                b.spawn(Effect("Defense Break".to_owned()));
            });
        });
    world
        .spawn((
            Unit("Lana".to_owned()),
            Weapon("Axe".to_owned()),
            Armor("Robe".to_owned()),
        ))
        .with_children(|b| {
            b.spawn(Potion("Fire Potion".to_owned()));
            b.spawn(Ability("Regenerate".to_owned()))
                .with_children(|b| {
                    b.spawn(Effect("Hp Restore".to_owned()));
                    b.spawn(Effect("Mp Restore".to_owned()));
                });
            b.spawn(Ability("Fire Ball".to_owned())).with_children(|b| {
                b.spawn(Effect("Burn Damage".to_owned()));
            });
        });

    let validation = json!([
        {
            "unit": "Bob",
            "weapon": null,
            "armor": null,
            "potions": [],
            "abilities": []
        },
        {
            "unit": "Eric",
            "weapon": "Sword",
            "armor": null,
            "potions": [
                "Hp Potion",
                "Mp Potion",
            ],
            "abilities": [
                {
                    "ability": "Thrust",
                    "effects": [
                        "Defense Break",
                    ]
                },
            ]
        },
        {
            "unit": "Lana",
            "weapon": "Axe",
            "armor": "Robe",
            "potions": [
                "Fire Potion"
            ],
            "abilities": [
                {
                    "ability": "Regenerate",
                    "effects": [
                        "Hp Restore",
                        "Mp Restore"
                    ]
                },
                {
                    "ability": "Fire Ball",
                    "effects": [
                        "Burn Damage"
                    ]
                },
            ]
        },
    ]);

    let value = world
        .save::<SerializeUnit, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<SerializeUnit>();
    assert_eq!(world.entities().len(), 0);

    world.load::<SerializeUnit, _>(&value).unwrap();

    let value = world
        .save::<SerializeUnit, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<SerializeUnit>();
    assert_eq!(world.entities().len(), 0);

    world.load::<SerializeUnit, _>(&validation).unwrap();

    let value = world
        .save::<SerializeUnit, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(value, validation);
}
