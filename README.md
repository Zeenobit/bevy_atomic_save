# ☢️ Bevy Atomic Save

An atomic save/load system for [Bevy](https://github.com/bevyengine/bevy).

## Features

- Save and load a [World](https://docs.rs/bevy/latest/bevy/ecs/world/struct.World.html) into a [RON](https://github.com/ron-rs/ron) file on disk
- Control which entities should participate in save/load operations
- Operations are synchronous, providing precise control over when save/load happens
- Dump feature useful for inspecting worlds in text format without any boilerplate.

## Overview

With the latest version of Bevy, it is possible to save the state of a world into a `DynamicScene` (see [example](https://github.com/bevyengine/bevy/blob/main/examples/scene/scene.rs)). While this approach is useful for scene management and editting, it's not practical to use the same approach for saving and loading the game state.

In most typical cases, a game needs to save only a minimal subset of the world to be able to resume its state from disk. Visual and aesthetic elements of the game such as UI, models, sprites, cameras, or logic systems do not need to be serialized as they are usually initialized during game start.

This crate solves this problem by providing a framework for marking entities which need to be saved and loaded, along with functions to save/load these entities into and from disk.

## Usage

### Save

1. Ensure `SavePlugin` is added to your `App`.
```rust
use bevy_atomic_save::SavePlugin;
...
app.add_plugin(SavePlugin);
```

2. Mark any entities which should be saved using the `Save` component. This may either be a `Bundle`, or inserted like a regular component. Entities marked for save should have components which derive `Reflect`. Any component which does not derive `Reflect` is not saved.
```rust
use bevy::prelude::*;
use bevy_atomic_save::Save;

#[derive(Bundle)]
struct PlayerBundle {
    /* ... Serializable Player Data ... */
    save: Save,
}
```
3. Save a world to disk using `SaveWorld` via a `&mut World` or `&mut Commands`.
```rust
use bevy::prelude::*;
use bevy_atomic_save::SaveWorld;

fn trigger_save(mut commands: Commands) {
    commands.save("world.ron");
}
```

### Load

1. Mark any entities which should be unloaded prior to load with `Unload`.
```rust
use bevy::prelude::*;
use bevy_atomic_save::Unload;

#[derive(Bundle)]
struct PlayerModelBundle {
    /* ... Player Transform, Mesh, Sprite, etc. ... */
    unload: Unload,
}
```
2. Load a previously saved file using `LoadWorld` via a `&mut World` or `&mut Commands`.<br/>
This starts a load process, which starts by deserializing the given file and then despawning (recursively) all entities marked with `Save` or `Unload` components. Finally, new entities are spawned and `SaveStage::PostLoad` begins.
```rust
use bevy::prelude::*;
use bevy_atomic_save::LoadWorld;

fn trigger_load(mut commands: Commands) {
    commands.load("world.ron");
}
```
3. Update entity references during `SaveStage::PostLoad`.<br/>
During load, there is no guarantee that the indices of saved entities are preserved. This is because there may already be entities in the current world with those indices, which cannot be despawned prior to load. Because of this, any components which reference entities should update their referenced entity during `SaveStage::PostLoad`.<br/>
This can be done by implementing the `FromLoaded` trait for any components which reference entities, and then registering those components in your `app` using `RegisterLoaded`.<br/>
See `./examples/pawn.rs` for a concrete example on how to do this.</br>
Alternatively, this can also be done manually by adding a system to `SaveStage::PostLoad` and reading the `Loaded` resource directly.<br/>
```rust
use bevy::prelude::*;
use bevy_atomic_save::{FromLoaded, RegisterLoaded};

#[derive(Component)]
struct SomeEntity(Entity);
impl FromLoaded for SomeEntity {
    fn load(&mut self, loaded: &Loaded) {
        self.0.load(loaded);
    }
}

...

app.register_loaded::<SomeEntity>();
```

## Notes

### Resources
Currently, a `DynamicScene` in Bevy does not save `Resource` items. To save/load resources, it is recommended to spawn your saved resources as entities with a `Save` component. This also gives you control over exactly which resources should be saved.

### Bevy Components and Entity References
Some components in Bevy reference entities (e.g. `Parent` and `Children`), which would need to update their references during `SaveStage::PostLoad`. This crate does **NOT** provide this functionality. In most cases, you shouldn't need to save such components, as they typically belong to scene entities which may be spawned from loaded game data.

### World Dump
During development, it may be useful to examine a world in raw text format, within a specific frame, for diagnostics purposes. This crate provides a simple function to do this which uses the underlying save system to dump the world state into a RON file. See `SaveWorld::dump` for details.

The only difference between a dump and a save request is that a dump saves *all* entities, as opposed to save which only saves entities with a `Save` component.

The result of a dump should not be loaded, as it can result in duplicate entities afterwards.

## Future Plans
- Provide asynchronous options
