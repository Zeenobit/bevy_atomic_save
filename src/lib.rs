use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;

mod load;
mod plugin;
mod save;

pub use load::*;
pub use plugin::*;
pub use save::*;

#[derive(StageLabel)]
pub enum SaveStage {
    /// The [`Stage`] after [`CoreStage::Last`] during which [`World`] is saved.
    Save,
    /// The [`Stage`] before [`CoreStage::First`] during which [`World`] is loaded.
    Load,
    /// The [`Stage`] after [`SaveStage::Load`].
    ///
    /// This stage is typically reserved for any systems which handle [`Loaded`] entities.
    PostLoad,
}

/// Trait used to save a [`World`] to a file.
pub trait SaveWorld {
    /// Inserts a new [`Request::Save`] with the given `path` into this [`World`].
    ///
    /// This request is processed during [`SaveStage::Save`]. During this stage, any [`Entity`]
    /// with a [`Save`] [`Component`] is serialized into a [`DynamicScene`] which is then written
    /// into a file located at given `path`.
    ///
    /// If the save request fails, an [`error`] message will be logged with cause of failure.
    fn save(self, path: impl Into<PathBuf>);

    /// Inserts a new [`Request::Save`] with the given `path` into this [`World`].
    ///
    /// This request is processed during [`SaveStage::Save`]. During this stage, all entities
    /// are serialized into a [`DynamicScene`] which is then written into a file located at given `path`.
    ///
    /// Unlike [`SaveWorld::save()`], this function saves *all* entities and their serializable components.
    /// The resulting file should not be used for loading, as it most likely has more data than required to
    /// actually load the game, which may result in duplicate or conflicting entities.
    ///
    /// As a result, this function should be used primarily for diagnostics, as it can be useful for inspecting
    /// worlds in a very simple, and somewhat readable text format.
    ///
    /// If the dump request fails, an [`error`] message will be logged with cause of failure.
    fn dump(self, path: impl Into<PathBuf>);
}

impl SaveWorld for &mut Commands<'_, '_> {
    fn save(self, path: impl Into<PathBuf>) {
        self.insert_resource(Request::Save {
            path: path.into(),
            mode: SaveMode::Filtered,
        })
    }

    fn dump(self, path: impl Into<PathBuf>) {
        self.insert_resource(Request::Save {
            path: path.into(),
            mode: SaveMode::Dump,
        })
    }
}

impl SaveWorld for &mut World {
    fn save(self, path: impl Into<PathBuf>) {
        self.insert_resource(Request::Save {
            path: path.into(),
            mode: SaveMode::Filtered,
        })
    }

    fn dump(self, path: impl Into<PathBuf>) {
        self.insert_resource(Request::Save {
            path: path.into(),
            mode: SaveMode::Dump,
        })
    }
}

/// Trait used to load a [`World`] from a file.
pub trait LoadWorld {
    /// Inserts a new [`Request::Load`] from the given `path` for this [`World`].
    ///
    /// This request is processed during [`SaveStage::Load`]. During this stage, any [`Entity`]
    /// with a [`Save`] or [`Unload`] [`Component`] is despawned recursively. Then, entities are deserialized
    /// from the given path (which should point to a previously saved file) and spawned in this [`World`]
    /// with a new [`Loaded`] [`Component`]. This component is removed after [`SaveStage::PostLoad`].
    ///
    /// If the load request fails, an [`error`] message will be logged with cause of failure.
    ///
    /// After a successful load, there is no guarantee that a loaded entity will have the same index with which
    /// it was saved. This is because there may already be an entity with that index in this world which is
    /// not marked for [`Save`] or [`Unload`]. This means any saved entity references are most likely invalid
    /// after load.
    ///
    /// To solve this, during [`SaveStage::PostLoad`], systems may use the [`Loaded`] component to update entity
    /// references as required. See examples for how this would be done.
    fn load(self, path: impl Into<PathBuf>);
}

impl LoadWorld for &mut Commands<'_, '_> {
    fn load(self, path: impl Into<PathBuf>) {
        self.insert_resource(Request::Load { path: path.into() })
    }
}

impl LoadWorld for &mut World {
    fn load(self, path: impl Into<PathBuf>) {
        self.insert_resource(Request::Load { path: path.into() })
    }
}

pub enum SaveMode {
    /// Save entities with a [`Save`] component.
    Filtered,
    /// Save all entities.
    Dump,
}

/// A [`Resource`] used to trigger a save or load request.
#[derive(Resource)]
pub enum Request {
    Save { path: PathBuf, mode: SaveMode },
    Load { path: PathBuf },
}

impl Request {
    fn should_save(&self) -> bool {
        matches!(self, Self::Save { .. })
    }

    fn should_load(&self) -> bool {
        matches!(self, Self::Load { .. })
    }
}
