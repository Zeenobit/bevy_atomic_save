use super::*;

/// A [`Component`] which indicates that its [`Entity`] should be saved.
///
/// Any entity with a [`Save`] component is despawned during [`SaveStage::Load`].
#[derive(Component, Default)]
pub struct Save;

/// A [`RunCriteria`] which returns [`ShouldRun::Yes`] if there is a save [`Request`] present; [`ShouldRun::No`] otherwise.
pub fn should_save(request: Option<Res<Request>>) -> ShouldRun {
    match request.map(|request| request.should_save()) {
        Some(true) => ShouldRun::Yes,
        _ => ShouldRun::No,
    }
}

/// A [`System`] which handles a save [`Request`].
pub fn save(world: &mut World) {
    if let Some(Request::Save { path, mode }) = world.remove_resource::<Request>() {
        let entities: Vec<Entity> = match mode {
            SaveMode::Filtered => world
                .query_filtered::<Entity, With<Save>>()
                .iter(world)
                .collect(),
            SaveMode::Dump => world.iter_entities().collect(),
        };

        let scene = save_world(world, entities);
        match scene.serialize_ron(world.resource::<AppTypeRegistry>()) {
            Ok(serialized_scene) => match File::create(&path) {
                Ok(mut file) => match file.write_all(serialized_scene.as_bytes()) {
                    Ok(()) => info!("save successful: {path:?}"),
                    Err(why) => error!("save failed: {why:?}"),
                },
                Err(why) => {
                    error!("file creation failed: {why:?}");
                }
            },
            Err(why) => {
                error!("serialization failed: {why:?}");
            }
        }
    }
}

/// Saves the `entities` within the given [`World`] and returns it as a serializable [`DynamicScene`].
pub fn save_world(world: &World, entities: impl IntoIterator<Item = Entity>) -> DynamicScene {
    let mut scene_builder = DynamicSceneBuilder::from_world(world);
    scene_builder.extract_entities(entities.into_iter());
    scene_builder.build()
}
