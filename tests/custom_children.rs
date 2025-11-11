use std::convert::Infallible;

use bevy::ecs::{
    component::Component,
    entity::Entity,
    lifecycle::HookContext,
    world::{DeferredWorld, EntityWorldMut, World},
};
use bevy::reflect::TypePath;
use bevy_serde_lens::{BevyObject, Child, ChildVec, ChildrenLike, Maybe, WorldExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Component)]
#[component(on_remove = on_remove_hook_one::<C>)]
pub struct CustomChild<const C: char>(Entity);

fn on_remove_hook_one<const C: char>(mut world: DeferredWorld, cx: HookContext) {
    let Some(child) = world.entity(cx.entity).get::<CustomChild<C>>() else {
        return;
    };
    let entity = child.0;
    let mut commands = world.commands();
    commands.queue(move |w: &mut World| w.entity_mut(entity).despawn());
}

impl<const C: char> ChildrenLike for CustomChild<C> {
    fn add_child(mut parent: EntityWorldMut, child: Entity) -> Result<(), impl std::fmt::Display> {
        if let Some(mut children) = parent.get_mut::<CustomChildren<C>>() {
            children.0.push(child);
        } else {
            parent.insert(CustomChild::<C>(child));
        }
        Ok::<_, Infallible>(())
    }

    fn iter_children(&self) -> impl Iterator<Item = Entity> {
        std::iter::once(self.0)
    }
}

#[derive(Debug, Component)]
#[component(on_remove = on_remove_hook::<C>)]
pub struct CustomChildren<const C: char>(Vec<Entity>);

fn on_remove_hook<const C: char>(mut world: DeferredWorld, cx: HookContext) {
    let Some(children) = world.entity(cx.entity).get::<CustomChildren<C>>() else {
        return;
    };
    let v = children.0.clone();
    let mut commands = world.commands();
    commands.queue(move |w: &mut World| {
        for entity in v {
            w.entity_mut(entity).despawn()
        }
    });
}

impl<const C: char> ChildrenLike for CustomChildren<C> {
    fn add_child(mut parent: EntityWorldMut, child: Entity) -> Result<(), impl std::fmt::Display> {
        if let Some(mut children) = parent.get_mut::<CustomChildren<C>>() {
            children.0.push(child);
        } else {
            parent.insert(CustomChildren::<C>(vec![child]));
        }
        Ok::<_, Infallible>(())
    }

    fn iter_children(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }
}

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
    weapon: Maybe<Child<Weapon, CustomChild<'w'>>>,
    #[bevy_object(no_filter)]
    armor: Child<Armor, CustomChild<'a'>>,
    #[serde(default)]
    potions: ChildVec<Potion, CustomChildren<'P'>>,
    #[serde(default)]
    abilities: ChildVec<SerializeAbility, CustomChildren<'A'>>,
}

#[derive(BevyObject)]

pub struct SerializeAbility {
    ability: Ability,
    effects: ChildVec<Effect>,
}

#[test]
pub fn test_custom_hierarchy() {
    let mut world = World::new();

    let validation = json!([
        {
            "unit": "Bob",
            "weapon": null,
            "armor": "Plate",
            "potions": [],
            "abilities": []
        },
        {
            "unit": "Eric",
            "weapon": "Sword",
            "armor": "Leather",
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
    world.load::<SerializeUnit, _>(&validation).unwrap();
    assert_eq!(world.entities().len(), 18);

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
}
