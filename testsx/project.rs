use bevy_app::App;
use bevy_asset::{Asset, AssetApp, AssetPlugin, Assets, Handle};
use bevy_ecs::{component::Component, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{bind_object, NoContext, SerdeProject, WorldExtension};
use rustc_hash::FxHashMap;
use serde_json::json;

#[derive(Debug, PartialEq, Eq, Hash, TypePath, Asset)]
pub struct Int(i32);

impl SerdeProject for Int {
    type Ctx = NoContext;

    type Ser<'t> = i32;

    type De<'de> = i32;

    fn to_ser<'t>(&'t self, _: &<Self::Ctx as bevy_serde_lens::FromWorldAccess>::Ref<'t>) -> Result<Self::Ser<'t>, Box<bevy_serde_lens::Error>> {
        Ok(self.0)
    }

    fn from_de(_: &mut <Self::Ctx as bevy_serde_lens::FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, Box<bevy_serde_lens::Error>> {
        Ok(Self(de))
    }
}

use bevy_serde_lens::{ProjectOption, ProjectVec, ProjectMap};

#[derive(Component, SerdeProject)]
pub struct Thing {
    #[serde_project("ProjectOption<Int>")]
    optional: Option<Int>,

    #[serde_project("ProjectVec<Vec<Int>>")]
    vec: Vec<Int>,

    #[serde_project("ProjectMap<FxHashMap<Int, Int>>")]
    map: FxHashMap<Int, Int>,
}

use bevy_serde_lens::asset::UniqueHandle;

#[derive(Component, SerdeProject)]
pub struct ThingAsset {
    #[serde_project("ProjectOption<Handle<Int>, UniqueHandle<Int>>")]
    optional: Option<Handle<Int>>,

    #[serde_project("ProjectVec<Vec<Handle<Int>>, UniqueHandle<Int>>")]
    vec: Vec<Handle<Int>>,

    #[serde_project("ProjectMap<FxHashMap<Int, Handle<Int>>, Int, UniqueHandle<Int>>")]
    map: FxHashMap<Int, Handle<Int>>,
}


bind_object!(Thing as "thing");

bind_object!(ThingAsset as "thing");

#[test]
pub fn test() {
    let mut world = World::new();

    world.spawn(Thing{
        optional: Some(Int(1)),
        vec: vec![Int(2), Int(3), Int(4)],
        map: FxHashMap::from_iter([
            (Int(5), Int(6)),
            (Int(7), Int(8)),
            (Int(9), Int(10)),
        ]),
    });
    

    let validation = json!([
        {
            "optional": 1,
            "vec": [2, 3, 4],
            "map": {
                "5": 6,
                "7": 8,
                "9": 10,
            }
        },
    ]);

    let value = world.save::<Thing, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<Thing>();
    assert_eq!(world.entities().len(), 0);

    world.load::<Thing, _>(&value).unwrap();

    let value = world.save::<Thing, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);


    world.spawn(Thing{
        optional: None,
        vec: vec![],
        map: FxHashMap::default(),
    });
    

    let validation = json!([
        {
            "optional": 1,
            "vec": [2, 3, 4],
            "map": {
                "5": 6,
                "7": 8,
                "9": 10,
            }
        },
        {
            "optional": null,
            "vec": [],
            "map": {},
        },
    ]);

    let value = world.save::<Thing, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<Thing>();
    assert_eq!(world.entities().len(), 0);

    world.load::<Thing, _>(&value).unwrap();

    let value = world.save::<Thing, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

}

#[test]
pub fn test2() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Int>();
    
    let handles: Vec<_> = (0..11).map(|x| app.world.resource_mut::<Assets<Int>>().add(Int(x))).collect();

    app.world.spawn(ThingAsset{
        optional: Some(handles[1].clone()),
        vec: vec![handles[2].clone(), handles[3].clone(), handles[4].clone()],
        map: FxHashMap::from_iter([
            (Int(5), handles[6].clone()),
            (Int(7), handles[8].clone()),
            (Int(9), handles[10].clone()),
        ]),
    });
    

    let validation = json!([
        {
            "optional": 1,
            "vec": [2, 3, 4],
            "map": {
                "5": 6,
                "7": 8,
                "9": 10,
            }
        },
    ]);

    let mut world = app.world;

    let value = world.save::<ThingAsset, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<ThingAsset>();
    assert_eq!(world.entities().len(), 0);

    world.load::<ThingAsset, _>(&value).unwrap();

    let value = world.save::<ThingAsset, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);


    world.spawn(ThingAsset{
        optional: None,
        vec: vec![],
        map: FxHashMap::default(),
    });
    

    let validation = json!([
        {
            "optional": 1,
            "vec": [2, 3, 4],
            "map": {
                "5": 6,
                "7": 8,
                "9": 10,
            }
        },
        {
            "optional": null,
            "vec": [],
            "map": {},
        },
    ]);

    let value = world.save::<ThingAsset, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

    world.despawn_bound_objects::<ThingAsset>();
    assert_eq!(world.entities().len(), 0);

    world.load::<ThingAsset, _>(&value).unwrap();

    let value = world.save::<ThingAsset, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, validation);

}