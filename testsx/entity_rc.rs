// Shows how to create an `Rc` of entity relationship.
use std::sync::Arc;

use bevy_ecs::{component::Component, entity::Entity, system::{Commands, Query, RunSystemOnce}, world::{EntityWorldMut, World}};
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_serde_lens::{bind_object, entity::EntitySmartPointer, BevyObject, WorldExtension};
use bevy_serde_lens_derive::SerdeProject;
use bevy_serde_lens::entity::EntityPointer;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Component, Clone)]
pub struct EntityRc {
    entity: Entity,
    pointer: Arc<()>,
}

#[derive(Debug, Component)]
pub struct EntityRcDrop(Arc<()>);

pub trait EntityRcMaker {
    fn make_rc(&mut self) -> EntityRc;
}

impl EntityRcMaker for EntityWorldMut<'_> {
    fn make_rc(&mut self) -> EntityRc {
        let shared = Arc::new(());
        let drop = EntityRcDrop(shared.clone());
        self.insert(drop);
        EntityRc {
            entity: self.id(),
            pointer: shared,
        }
    }
}

impl<B: BevyObject> EntitySmartPointer<B> for EntityRc {
    type Pointee = EntityRcDrop;

    fn from_entity(entity: Entity) -> Self {
        EntityRc {
            entity,
            pointer: Arc::new(()),
        }
    }

    fn get_entity(&self) -> Entity {
        self.entity
    }

    fn inject_pointee(&mut self) -> Self::Pointee {
        EntityRcDrop(self.pointer.clone())
    }
}

pub fn drop_entity_pointee(
    mut commands: Commands,
    query: Query<(Entity, &EntityRcDrop)>
) {
    for (entity, pointee) in query.iter() {
        if Arc::strong_count(&pointee.0) < 2 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Debug, Component, SerdeProject)]
pub struct EntityNumber {
    #[serde_project("EntityPointer::<Number>")]
    entity: Entity,
    number: i32,
}


#[derive(Debug, Component, SerdeProject)]
pub struct EntityNumberRc {
    #[serde_project("EntityPointer::<Number, EntityRc>")]
    entity: EntityRc,
    number: i32,
}

#[derive(Debug, Component, Serialize, Deserialize)]
pub struct Number {
    number: i32,
}

bind_object!(EntityNumber as "EntityNumber");
bind_object!(EntityNumberRc as "EntityNumberRc");

#[test]
pub fn test1() {
    let mut world = World::new();
    let number = world.spawn(Number { number: 69 }).id();
    world.spawn(EntityNumber {
        entity: number,
        number: 420
    });
    
    let validation = json!([
        {
            "entity": {
                "number": 69
            },
            "number": 420
        },
    ]);

    let value = world.save::<EntityNumber, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<EntityNumber>();
    // Does not despawn the pointed object (unfortunately)
    assert_eq!(world.entities().len(), 1);

    world.load::<EntityNumber, _>(value).unwrap();
    assert_eq!(world.entities().len(), 3);

    let value = world.save::<EntityNumber, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);
}

#[test]
pub fn test2() {
    let mut world = World::new();
    let number = world.spawn(Number { number: 69 }).make_rc();
    world.spawn(EntityNumberRc {
        entity: number,
        number: 420
    });
    
    let validation = json!([
        {
            "entity": {
                "number": 69
            },
            "number": 420
        },
    ]);

    let value = world.save::<EntityNumberRc, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<EntityNumberRc>();
    // Does not despawn the pointed object
    assert_eq!(world.entities().len(), 1);

    world.run_system_once(drop_entity_pointee);

    assert_eq!(world.entities().len(), 0);

    world.load::<EntityNumberRc, _>(value).unwrap();
    assert_eq!(world.entities().len(), 2);

    let value = world.save::<EntityNumberRc, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);
}