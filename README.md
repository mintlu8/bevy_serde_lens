# bevy_serde_lens

[![Crates.io](https://img.shields.io/crates/v/bevy_serde_lens.svg)](https://crates.io/crates/bevy_serde_lens)
[![Docs](https://docs.rs/bevy_serde_lens/badge.svg)](https://docs.rs/bevy_serde_lens/latest/bevy_serde_lens/)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/)

Stateful, structural and human-readable serialization crate for the bevy engine.

## Features

* Stateful serialization and deserialization with world access.
* Treat an `Entity`, its `Component`s and children as a single serde object.
* Deserialize trait objects like `Box<dyn T>`, as an alternative to `typetag`.
* Extremely lightweight and modular. No systems, no plugins.
* Supports every serde format using familiar syntax.
* Serialize `Handle`s and provide a generalized data interning interface.
* Serialize stored `Entity`s in a safe manner.

## Getting Started

Imagine we want to Serialize an `Entity` Character with some components and children.

```rust
bind_object!(pub struct SerializeCharacter as (With<Character>, Without<NPC>) {
    character: Character,
    position: Position,
    #[serde(default)]
    weapon: Maybe<Weapon>,
    #[serde(default)]
    shield: Maybe<Shield>,
    #[serde(default)]
    potions: ChildVec<Potion>,
})
```

This creates a `BevyObject` that marks entities that satisfies a specific `QueryFilter` as serializable.

Then call `save` on `World`, where `serializer` is something like `serde_json::Serializer`.

```rust
// Save
world.save::<Character>(serializer)
// Load
world.load::<Character>(deserializer)
```

If you prefer more familiar syntax like

```rust
serde_json::to_string(..)
```

You can create a `SerializeLens`:

```rust
// `SerializeLens` has a reference to `World` and implements `Serialize`
let lens = world.serialize_lens::<Character>();
serde_json::to_string(&lens);
// This signature works because the world is stored as a thread local
world.deserialize_scope(|| {
    // Return object doesn't matter, data is stored in the world
    let _ = serde_json::from_str::<InWorld<Character>>(&my_string);
    let _ = serde_json::from_str::<InWorld<Monster>>(&my_string2);
})
```

This saves a list of Characters as an array:

```rust
[
    { .. },
    { .. },
    ..
]
```

To delete all associated entities:

```rust
// Despawn all character.
world.despawn_bound_objects::<Character>()
```

To save multiple types of objects in a batch, create a batch serialization type with the `batch!` macro.

```rust
type SaveFile = batch!(
    Character, Monster, Terrain,
    // Use `SerializeResource` to serialize a resource.
    SerializeResource<MyResource>,
);
world.save::<SaveFile>(serializer)
world.load::<SaveFile>(deserializer)
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

## Archetypal Binding

`bind_query!` can be used to speed up serialization.
By default `bind_object` queries the world hierarchically as a tree,
This has roughly similar performance to building a `DynamicScene`
then serializing it (with `ron`).
If your binding has no `Child` or `ChildVec`, you can use `bind_query!`
which serializes a `Query` directly.

## Projection Types

The crate provides various projection types for certain common use cases.

For example, to serialize a `Handle` as its string path,
you can use `#[serde(with = "PathHandle")]` like so

```rust
#[derive(Serialize, Deserialize)]
struct MySprite {
    #[serde(with = "PathHandle")]
    image: Handle<Image>
}
```

Or use the newtype directly.

```rust
#[derive(Serialize, Deserialize)]
struct MySprite {
    image: PathHandle<Image>
}
```

## EntityId

We provide a framework to serialize `Entity`, `Parent` etc. Before we start keep in mind
this only works with a serializer that can preserve the order of maps.
In `serde_json` for instance, you must enable feature `preserve_order` to use these features.

When using `bind_object!` or `bind_query!`, you can specify the `EntityId` component.
This registers the `Entity` id of this entity for future use in the same batch.

**After** the entry, future entities can use `Parented` to parent to this entity,
or use `EntityPtr` to serialize an `Entity` that references this entity.
`Parented` can be used in `bind_query` which might made it more performant than `ChildVec`.

## TypeTag

The `typetag` crate allows you to serialize trait objects like `Box<dyn T>`,
but using `typetag` will always
pull in all implementations linked to your build and does not work on WASM.
To address these limitations this crate allows you to register deserializers manually
in the bevy `World` and use the `TypeTagged` projection type for serialization.

```rust
world.register_typetag::<Box<dyn Animal>, Cat>()
```

then

```rust
#[derive(Serialize, Deserialize)]
struct MyComponent {
    #[serde(with = "TypeTagged")]
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

| bevy | bevy-serde-lens    |
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
