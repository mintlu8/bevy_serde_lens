# bevy_serde_lens

[![Crates.io](https://img.shields.io/crates/v/bevy_serde_lens.svg)](https://crates.io/crates/bevy_serde_lens)
[![Docs](https://docs.rs/bevy_serde_lens/badge.svg)](https://docs.rs/bevy_serde_lens/latest/bevy_serde_lens/)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/)

Blazingly fast, schema based human-readable serialization crate for the bevy engine.

## Features

* Stateful serialization and deserialization with world access.
* No systems, no plugins.
* Blazingly fast (compared to `DynamicScene`).
* Treat an `Entity`, its `Component`s and children as a single serde object.
* Deserialize trait objects like `Box<dyn T>`, as an alternative to `typetag`.
* Supports every serde format using familiar syntax.
* Serialize `Handle`s and provide a generalized data interning interface.
* No reflection needed.

## Getting Started

Imagine we have a typical `Character` bundle.

First we derive `BevyObject`:

```rust
#[derive(Bundle, BevyObject)]
#[bevy_object(query)]
pub struct Character {
    pub transform: Transform,
    pub name: Name,
    pub position: Position,
    pub hp: Hp,
}
```

* `#[bevy_object(query)]`

This indicates we are serializing a query instead of a hierarchical tree, which improves performance.

To serialize we simply do:

```rust
serde_json::to_string(&world.serialize_lens::<Character>());
```

This finds all entities that fits the `QueryFilter` of the bundle and serializes them in an array.

To deserialize we use `deserialize_scope`:

```rust
world.deserialize_scope(|| {
    // Returned object doesn't matter, data is stored in the world.
    let _ = serde_json::from_str::<InWorld<Character>>(&json_string);
})
```

This statement spawns new entities in the world and fills them with deserialized data.

You might want to delete current entities before loading new ones,
to delete all associated entities of a serialization:

```rust
// Despawn all character.
world.despawn_bound_objects::<Character>()
```

To save multiple types of objects in a batch, create a batch serialization type with the `batch!` macro.

```rust
type SaveFile = batch!(
    Character, Monster,
    // Use `SerializeResource` to serialize a resource.
    SerializeResource<Terrain>,
);
world.serialize_lens::<SaveFile>()
world.deserialize_scope(|| {
    let _ = serde_json::from_str::<InWorld<SaveFile>>(&json_string);
})
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
    "Terrain": ..
}
```

## Advanced Serialization

`BevyObject` is not just a clone of `Bundle`, we support additional types.

* `impl BevyObject`: Components are automatically `BevyObject` and `BevyObject` can contain multiple other `BevyObject`s.
* `Maybe<T>` can be used if an item may or may not exist.
* `DefaultInit` initializes a non-serialize component with `FromWorld`.
* `Child<T>` finds and serializes a single `BevyObject` in children.
* `ChildVec<T>` finds and serializes multiple `BevyObject`s in children.

New in 0.5:

* `Child<T, C>` finds and serializes a single `BevyObject` from a custom children component.
* `ChildVec<T, C>` finds and serializes multiple `BevyObject`s from a custom children component.

See the `BevyObject` derive macro for more details.

```rust
// Note we cannot derive bundle anymore.
// #[bevy_object(query)] also cannot be used due to children being serialized.
#[derive(BevyObject)]
#[bevy_object(rename = "character")]
pub struct Character {
    pub transform: Transform,
    pub name: Name,
    pub position: Position,
    pub hp: Hp,
    #[serde(default)]
    pub weapon: Maybe<Weapon>
    #[serde(skip)]
    pub cache: DefaultInit<Cache>,
    pub potions: ChildVec<Potion>
}
```

## Stateful Serialization

When using `bevy_serde_lens` you can use `with_world` to access `&World`
in `Serialize` implementations and `with_world_mut` to access `&mut World`
in `Deserialize` implementations.

These functions actually comes from `bevy_serde_lens_core`
which is more semver stable and more suited as a dependency for library authors.

## Serialize Handles

To serialize a `Handle` as its string path, you can use `#[serde(with = "PathHandle")]`.
To serialize its content, use `#[serde(with = "UniqueHandle")]`.

```rust
#[derive(Component, Serialize, Deserialize)]
struct MySprite {
    #[serde(with = "PathHandle")]
    image: Handle<Image>
}
```

Or use the newtype directly.

```rust
#[derive(Component, Serialize, Deserialize)]
struct MySprite {
    image: PathHandle<Image>
}
```

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
    weapon: Box<dyn Animal>
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

| bevy | bevy-serde-lens-core | bevy-serde-lens    |
|------|----------------------|--------------------|
| 0.13 | -                    | 0.1-0.3            |
| 0.14 | 0.14                 | 0.4                |
| 0.15 | 0.15                 | 0.5-latest         |

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
