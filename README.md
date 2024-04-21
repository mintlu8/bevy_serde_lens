# bevy_serde_project

[![Crates.io](https://img.shields.io/crates/v/bevy_serde_project.svg)](https://crates.io/crates/bevy_serde_project)
[![Docs](https://docs.rs/bevy_serde_project/badge.svg)](https://docs.rs/bevy_serde_project/latest/bevy_serde_project/)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/)

Stateful, structural and human-readable serialization crate for the bevy engine.

## Features

* Stateful serialization and deserialization with world access.
* Treat an `Entity`, its `Component`s and children as a single serde object.
* Serialize `Handle`s and provide a generalized data interning interface.
* Serialize stored `Entity`s like smart pointers.
* Deserialize trait objects like `Box<dyn T>`, as an alternative to `typetag`.
* Extremely lightweight and modular. No systems, no plugins.
* Supports almost every serde format<sup>*</sup>

<sub>* Ergonomics may vary depend on `Serializer`, `Deserializer` and `DeserializeSeed` trait support</sub>

## Getting Started

Serialize an `Entity` Character with some components and children,
assuming all components are `Serialize` and `DeserializeOwned`:

```rust
bind_object!(Character as "Character" {
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
world.despawn_bound_objects::<Character>()
```

This saves a list of Characters as an array:

```rust
[
    { .. },
    { .. },
    ..
]
```

To save multiple types of objects in a batch, create a batch serialization type with the `batch!` macro.

```rust
type SaveFile = batch!(
    Character, Monster, Terrain,
    // Use `BindResource` to serialize a resource.
    BindResource<MyResource>,
);
world.save::<SaveFile>(serializer)
world.load::<SaveFile>(serializer)
world.despawn_bound_objects::<SaveFile>()
```

This saves each type in a map entry:

```rust
{
    "Character": [ 
        { .. },
        { .. },
        ..
    ],
    "Monster": [ .. ],
    "Terrain": [ .. ],
    "MyResource": ..
}
```

## FAQ

* What if my types aren't `Serialize` and `DeserializeOwned`?

We can derive or implement `SerdeProject` to convert them into `serde` types.

* I don't own the type

Use `Convert` and the `SerdeProject` macro to cast the type to an owned newtype.

* I have an ID and I want to serialize its content

`SerdeProject` allows you to fetch a resource from the world during serialization.

* I have a `Box<dyn T>`

If you are on a non-wasm platform you can try the `typetag` crate. If not,
or if you want more control, checkout the `typetagged` module in this crate.

## The traits and what they do

* `Serialize` and `DeserializeOwned`

Any `Serialize` and `DeserializeOwned` type is automatically `SerdeProject` and
any such `Component` is automatically a `BevyObject`.

This comes with the downside that we cannot implement `SerdeProject` on any foreign
type due to the orphan rule.
This is where `Convert` and the `SerdeProject`
macro comes in handy.

* `SerdeProject`

`SerdeProject` projects non-serde types into serde types with world access.

The `SerdeProject` macro implements
`SerdeProject` on type where all fields either implements `SerdeProject` or converts
to a `SerdeProject` newtype via the `Convert` trait.

### Example

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

* `Convert`

Convert allows you to `RefCast` a non-serializable type
to a newtype that implements `SerdeProject`.

For example `PathHandle<Handle<T>>` serializes `Handle` as a `String`, while
`UniqueHandle<Handle<T>>` serializes `Handle` as a `T`.
This zero-cost conversion can be done via the `ref_cast` crate.

* `BevyObject`

A `BevyObject` allows an `Entity` to be serialized. This can either be just a component,
or a combination of components, children, components on children, etc.

All `SerdeProject` components are `BevyObject`s.

* `BindBevyObject`

`BindBevyObject` is a `QueryFilter`, usually a key component,
that determines the entry point for serialization and deserialization.

Any entity that has the `QueryFilter` but does not satisfy the layout of the bound `BevyObject`
will result in an error.

use the `bind_object!` macro to create a serialization entry.

* `Named`

Provides a serialization name for resources.

* `SaveLoad`

Represents a batch serialization type, or contents of a single save file.

## TypeTag

The `typetag` crate allows you to serialize trait objects like `Box<dyn T>`,
but using `typetag` will always
pull in all implementations linked to your build and does not work on WASM.
To address these limitations this crate allows you to register deserializers manually
in the bevy `World` and use the `TypeTagged` newtype for serialization.

```rust
world.register_typetag::<Box<dyn Animal>, Cat>()
```

then

```rust
#[derive(SerdeProject)]
struct MyComponent {
    #[serde_project("TypeTagged<Box<dyn Weapon>>")]
    weapon: Box<dyn Weapon>
}
```

To have user friendly configuration files,
you can use `register_deserialize_any` and `AnyTagged` to allow `deserialize_any`, i.e.
deserialize `42` instead of `{"int": 42}` in self-describing formats.
Keep in mind using `AnyTagged` in a non-self-describing format like `postcard` will always return an error
as this is a limitation of the serde specification.

```rust
world.register_deserialize_any(|s: &str| 
    Ok(Box::new(s.parse::<Cat>()
        .map_err(|e| e.to_string())?
    ) as Box<dyn Animal>)
)
```

## Versions

| bevy | bevy-serde-project |
|------|--------------------|
| 0.13 | latest             |

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
