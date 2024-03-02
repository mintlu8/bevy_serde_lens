use bevy_ecs::{component::Component, world::World};
use bevy_hierarchy::BuildWorldChildren;
use bevy_serde_project::{bind_object, ChildVec, Maybe, Object, SerdeProject, WorldExtension};
use serde_json::json;

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct Unit(String);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct Weapon(String);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct Armor(String);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct Potion(String);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct Ability(String);

#[derive(SerdeProject, Component)]
#[serde(transparent)]
pub struct Effect(String);

bind_object!(Unit as "Unit" {
    unit => Unit,
    #[serde(default, skip_serializing_if="Option::is_none")]
    weapon => Maybe<Weapon>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    armor => Maybe<Armor>,
    #[serde(default, skip_serializing_if="Vec::is_empty")]
    potions => ChildVec<Potion>,
    #[serde(default, skip_serializing_if="Vec::is_empty")]
    abilities => ChildVec<Object<Ability>>
});

bind_object!(Ability as "Ability" {
    ability => Ability,
    effects => ChildVec<Effect>
});

#[test]
pub fn test() {
    let mut world = World::new();
    world.spawn(Unit("Bob".to_owned()));
    world.spawn((
        Unit("Eric".to_owned()),
        Weapon("Sword".to_owned()),
    )).with_children(|b| {
        b.spawn(Potion("Hp Potion".to_owned()));
        b.spawn(Potion("Mp Potion".to_owned()));
        b.spawn(Ability("Thrust".to_owned())).with_children(|b| {
            b.spawn(Effect("Defense Break".to_owned()));
        });
    });
    world.spawn((
        Unit("Lana".to_owned()),
        Weapon("Axe".to_owned()),
        Armor("Robe".to_owned()),
    )).with_children(|b| {
        b.spawn(Potion("Fire Potion".to_owned()));
        b.spawn(Ability("Regenerate".to_owned())).with_children(|b| {
            b.spawn(Effect("Hp Restore".to_owned()));
            b.spawn(Effect("Mp Restore".to_owned()));
        });
        b.spawn(Ability("Fire Ball".to_owned())).with_children(|b| {
            b.spawn(Effect("Burn Damage".to_owned()));
        });
    });


    let validation = json!([
        {
            "unit": "Bob"
        },
        {
            "unit": "Eric",
            "weapon": "Sword",
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

    let value = world.save::<Unit, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<Unit>();
    assert_eq!(world.entities().len(), 0);

    world.load::<Unit, _>(&value).unwrap();

    let value = world.save::<Unit, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

}