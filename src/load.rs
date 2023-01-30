use bevy::ecs::entity::EntityMap;
use bevy::scene::serde::SceneDeserializer;
use bevy::utils::HashMap;
use ron::Deserializer;
use serde::de::DeserializeSeed;

use super::*;

/// A [`Component`] which indicates that its [`Entity`] and all of its [`Children`] should be despawned before load.
///
/// Any entity with an [`Unload`] component is despawned during [`SaveStage::Load`].
#[derive(Component, Default)]
pub struct Unload;

/// A [`Resource`] available during [`SaveStage::PostLoad`] which contains a mapping of previously saved entities
/// to new loaded entities.
///
/// # Usage
///
/// When a world is loaded, there can be no guarantee that the index of loaded entities match
/// their saved values. This is because there may already be entities spawned with those indices
/// that do not interact with the save system. This can cause issues with any loaded components which
/// reference entities, since all saved references are invalidated upon load.
///
/// To solve this, the [`Loaded`] resource may be used to update any entity references
/// during [`SaveStage::PostLoad`]. This resource is added to world during this stage and
/// it contains the previously saved index of the loaded entities. Any type which references entities
/// can update its references using this resource.
///
/// This can be done more conveniently by implementing the [`FromLoaded`] trait for components which
/// reference entities.
///
/// [`Resource`]: bevy::prelude::Resource
#[derive(Resource)]
pub struct Loaded(HashMap<u32, Entity>);

impl Loaded {
    pub fn entity(&self, entity: Entity) -> Option<Entity> {
        self.0.get(&entity.index()).copied()
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
        error!("world write failed: {why:?}");
    }
    let mut loaded = HashMap::new();
    // TODO: EntityMap doesn't implement `iter()`
    for old_entity in entity_map.keys() {
        let entity = entity_map.get(old_entity).unwrap();
        debug!("entity update required: {old_entity:?} -> {entity:?}");
        loaded.insert(old_entity.index(), entity);
        world.entity_mut(entity).insert(Save);
    }
    world.insert_resource(Loaded(loaded));
}

/// A [`System`] which finalizes load process by removing [`Loaded`] components and consuming the [`Request`].
pub(crate) fn finish_load(mut commands: Commands) {
    commands.remove_resource::<Request>();
    commands.remove_resource::<Loaded>();
}

/// A [`System`] which despawns all entities with [`Save`] and [`Unload`] before load.
fn unload_world(world: &mut World) {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<Save>, With<Unload>)>>()
        .iter(world)
        .collect();
    for entity in entities {
        // Check the entity again in case it was despawned recursively
        if let Some(entity) = world.get_entity_mut(entity) {
            entity.despawn_recursive();
        }
    }
}

/// Trait used to read and update entity references from [`Loaded`].
///
/// # Usage
/// 
/// Components which implement this trait must be registered using [`RegisterLoaded`].
///
/// Use this trait to update references to entities during [`SaveStage::PostLoad`].
/// This trait is implemented for `Entity`, and `Option<Entity>`. This can be used to recursively
/// call [`FromLoaded::from_loaded`] on any entity references which need to be updated.
///
/// See [`Loaded`] for more details.
///
/// # Example
/// ```
/// # use bevy::prelude::*;
/// # use bevy_atomic_save::FromLoaded;
/// #[derive(Component)]
/// struct SomeEntity(Entity);
///
/// impl FromLoaded for SomeEntity {
///     fn load(&mut self, loaded: &Loaded) {
///         self.0.load(loaded);
///     }
/// }
/// ```
pub trait FromLoaded {
    fn from_loaded(&mut self, loaded: &Loaded);
}

impl FromLoaded for Entity {
    fn from_loaded(&mut self, loaded: &Loaded) {
        *self = loaded.entity(*self).expect("loaded entity is not valid");
    }
}

impl FromLoaded for Option<Entity> {
    fn from_loaded(&mut self, loaded: &Loaded) {
        if let Some(entity) = self {
            entity.from_loaded(loaded);
        }
    }
}

/// Extension trait used to register components which implement [`FromLoaded`] with an [`App`].
///
/// [`App`]: bevy::prelude::App
pub trait RegisterLoaded {
    /// Adds a system which calls [`FromLoaded::from_loaded`] on all instances of a component during [`SaveStage::PostLoad`].
    fn register_loaded<T: FromLoaded + Component>(self) -> Self;
}

impl RegisterLoaded for &mut App {
    fn register_loaded<T: FromLoaded + Component>(self) -> Self {
        self.add_system_to_stage(
            SaveStage::PostLoad,
            move |mut query: Query<&mut T>, loaded: Res<Loaded>| {
                for mut component in &mut query {
                    component.from_loaded(&loaded);
                }
            },
        )
    }
}
