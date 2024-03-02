# bevy_serde_project

[![Crates.io](https://img.shields.io/crates/v/bevy_serde_project.svg)](https://crates.io/crates/bevy_serde_project)
[![Docs](https://docs.rs/bevy_rectray/badge.svg)](https://docs.rs/bevy_serde_project/latest/bevy_serde_project/)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/)

A pretty and structural serialization crate for the bevy engine.

## Features

* Stateful serialization and deserialization with world access.
* Treat an `Entity`, its `Component`s and children as a single serde object.
* Serialize `Handle`s and provide a generalized data interning interface.
* Deserialize trait objects like `Box<dyn T>`, as an alternative to `typetag`.

## Getting Started

Assume all components are `Serialize` and `DeserializeOwned`.

Serialize an `Entity` Character with some components and children:

```rust
bind_object!(Character {
    #[serde(flatten)]
    character: Character,
    position: Position,
    #[serde(default, skip_serializing_if="Option::is_none")]
    weapon: Maybe<Weapon>,
    shield: Maybe<Shield>,
    #[serde(default, skip_serializing_if="Vec::is_empty")]
    potions: ChildVec<Potion>,
})
```

Then call `save` on `World`, where `serializer` is something like `serde_json::Serializer`.

```rust
// Save
world.save::<Character>(serializer)
// Load
world.load::<Character>(deserializer)
// Delete
world.despawn_bound_objects::<Character>(deserializer)
```

This saves a list of Characters like so:

```rust
[
    { .. },
    { .. },
    ..
]
```

To save multiple types of objects in a batch, create a batch serialization type with the `batch!` macro.

```rust
type SaveFileOne = batch!(Character, Monster, Terrain);
world.save::<SaveFileOne>(serializer)
world.load::<SaveFileOne>(serializer)
world.despawn_bound_objects::<SaveFileOne>(serializer)
```

This saves a map like so:

```rust
{
    "Character": [ 
        { .. },
        { .. },
        ..
    ],
    "Monster": [ .. ],
    "Terrain": [ .. ]
}
```

## The traits and what they do

### `Serialize` and `DeserializeOwned`

Any `Serialize` and `DeserializeOwned` type is automatically `SerdeProject` and
any such `Component` is automatically a `BevyObject`.

This comes with the downside that we cannot implement `SerdeProject` on any foreign
type due to the orphan rule.
This is where `Convert` and the `SerdeProject`(bevy_serde_project_derive::SerdeProject)
macro comes in handy.

### `FromWorldAccess`

A convenient trait for getting something from the world.

Either `NoContext`,
a `Resource` or `WorldAccess` (`&world` and `&mut World`)

### `SerdeProject`

`SerdeProject` projects non-serde types into serde types with world access.

The `SerdeProject`(bevy_serde_project_derive::SerdeProject) macro implements
`SerdeProject` on type where all fields either implements `SerdeProject` or converts
to a `SerdeProject` newtype via the `Convert` trait.

#### Example

Serialize a `Handle` as its path, stored in `AssetServer`.

```rust
#[derive(SerdeProject)]
struct MySprite {
    // implements serde, therefore is `SerdeProject`.
    pub name: String,
    // Calls `Convert` and `PathHandle<Image>` is `SerdeProject`.
    #[serde_project("PathHandle<Image>")]
    pub handle: Handle<Image>
}
```

### `BevyObject`

A `BevyObject` allows an `Entity` to be serialized.
All `SerdeProject` `Component`s are `BevyObject`s
since each entity can only have at most one of each component.

### `BindBevyObject`

`BindBevyObject` is a key `Component` that indicates an Entity is the `BevyObject`.
Any entity that has the `Component` but does not satisfy the layout of the `BevyObject`
will result in an error.
