use bevy::ecs::entity::EntityMap;
use bevy::scene::serde::SceneDeserializer;
use ron::Deserializer;
use serde::de::DeserializeSeed;

use super::*;

/// A [`Component`] which indicates that its [`Entity`] and all of its [`Children`] should be despawned before load.
///
/// Any entity with an [`Unload`] component is despawned during [`SaveStage::Load`].
#[derive(Component, Default)]
pub struct Unload;

/// A [`Component`] which indicates that its [`Entity`] has just been loaded.
///
/// # Usage
///
/// When a world is loaded, there can be no guarantee that the index of loaded entities match
/// their saved values. This is because there may already be entities spawned with those indices
/// that do not interact with the save system. This can cause issues with any loaded components which
/// reference entities, since all saved references are invalidated upon load.
///
/// To solve this, the [`Loaded`] component may be used to update any entity references
/// during [`SaveStage::PostLoad`]. This component is added to every loaded entity during this stage and
/// it contains the previously saved index of the loaded entity. Any component which references this old
/// index can then safely update its reference to point to this component's current, loaded entity.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Loaded(u32);

impl Loaded {
    /// Returns the raw index of the old entity from which this new entity was loaded from.
    pub fn index(&self) -> u32 {
        self.0
    }
}

/// A [`RunCriteria`] which returns [`ShouldRun::Yes`] if there is a load [`Request`] present; [`ShouldRun::No`] otherwise.
pub fn should_load(request: Option<Res<Request>>) -> ShouldRun {
    match request.map(|request| request.should_load()) {
        Some(true) => ShouldRun::Yes,
        _ => ShouldRun::No,
    }
}

/// A [`System`] which handles a load [`Request`] and starts the load process.
pub fn load(world: &mut World) {
    if let Request::Load { path } = world.resource::<Request>() {
        match File::open(path) {
            Ok(mut file) => {
                let mut serialized_scene = Vec::new();
                if let Err(why) = file.read_to_end(&mut serialized_scene) {
                    error!("file read failed: {why:?}");
                }
                match Deserializer::from_bytes(&serialized_scene) {
                    Ok(mut deserializer) => {
                        let result = SceneDeserializer {
                            type_registry: &world.resource::<AppTypeRegistry>().read(),
                        }
                        .deserialize(&mut deserializer);
                        match result {
                            Ok(scene) => {
                                unload_world(world);
                                load_world(world, scene);
                            }
                            Err(why) => {
                                error!("deserialization failed: {why:?}");
                            }
                        }
                    }
                    Err(why) => {
                        error!("deserializer creation failed: {why:?}");
                    }
                }
            }
            Err(why) => {
                error!("load failed: {why:?}");
            }
        }
    }
}

/// Loads a previously saved [`DynamicScene`] into the given [`World`].
pub fn load_world(world: &mut World, scene: DynamicScene) {
    unload_world(world);
    let mut entity_map = EntityMap::default();
    if let Err(why) = scene.write_to_world(world, &mut entity_map) {
        panic!("world write failed: {why:?}");
    }
    // TODO: EntityMap doesn't implement `iter()`
    for old_entity in entity_map.keys() {
        let entity = entity_map.get(old_entity).unwrap();
        info!("entity update required: {old_entity:?} -> {entity:?}");
        world
            .entity_mut(entity)
            .insert(Save)
            .insert(Loaded(old_entity.index()));
    }
}

/// A [`System`] which finalizes load process by removing [`Loaded`] components and consuming the [`Request`].
pub(crate) fn finish_load(query: Query<Entity, With<Loaded>>, mut commands: Commands) {
    for entity in &query {
        commands.entity(entity).remove::<Loaded>();
    }
    commands.remove_resource::<Request>();
}

/// A [`System`] which despawns all entities with [`Save`] and [`Unload`] before load.
fn unload_world(world: &mut World) {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<Save>, With<Unload>)>>()
        .iter(world)
        .collect();
    for entity in entities {
        // Check the entity again in case it was despawned recursively
        if world.get_entity(entity).is_some() {
            world.entity_mut(entity).despawn_recursive();
        }
    }
}
