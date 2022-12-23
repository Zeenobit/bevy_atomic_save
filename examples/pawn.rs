use bevy::{prelude::*, utils::HashMap};
use bevy_atomic_save::{LoadWorld, Loaded, Save, SavePlugin, SaveStage, SaveWorld, Unload};

fn main() {
    // Save
    {
        let mut app = app();

        let weapon = app.world.spawn(WeaponBundle::default()).id();
        let position = Vec2::new(4.0, 7.0);
        let pawn = app.world.spawn(PawnBundle::new(weapon, position)).id();
        app.update();

        // Pre-condition
        {
            let world = &app.world;
            assert!(world.entity(pawn).contains::<Sprite>());
            assert_eq!(
                world
                    .entity(pawn)
                    .get::<CurrentWeapon>()
                    .expect("pawn must have CurrentWeapon")
                    .entity()
                    .expect("pawn must have a weapon"),
                weapon,
                "pawn must have the correct weapon"
            );
        }

        app.world.save("pawn.ron");
        app.update();
    }

    // Load
    {
        let mut app = app();

        app.world.load("pawn.ron");
        app.update();

        let pawn = app
            .world
            .query_filtered::<Entity, With<Pawn>>()
            .single(&app.world);

        let weapon = app
            .world
            .query_filtered::<Entity, With<Weapon>>()
            .single(&app.world);

        // Post-condition
        {
            let world = &app.world;
            assert!(world.entity(pawn).contains::<Sprite>());
            assert_eq!(
                world
                    .entity(pawn)
                    .get::<CurrentWeapon>()
                    .expect("pawn must have CurrentWeapon")
                    .entity()
                    .expect("pawn must have a weapon"),
                weapon,
                "pawn must have the correct weapon"
            );
        }
    }
}

fn app() -> App {
    let mut app = App::new();
    // Minimum required plugins:
    app.add_plugins(MinimalPlugins).add_plugin(SavePlugin);

    // Register all saved types:
    app.register_type::<Pawn>()
        .register_type::<Position>()
        .register_type::<CurrentWeapon>()
        .register_type::<Option<Entity>>()
        .register_type::<Weapon>();

    // Game systems:
    app.add_startup_system(setup)
        .add_system(spawn_pawn_sprites)
        .add_system(update_model_position);

    // Post-load system to fix up entity references:
    app.add_system_to_stage(SaveStage::PostLoad, post_load);

    app
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Pawn;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Position(Vec2);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct CurrentWeapon(Option<Entity>);

impl CurrentWeapon {
    fn entity(&self) -> Option<Entity> {
        self.0
    }
}

#[derive(Bundle, Default)]
struct PawnBundle {
    // Pawn data:
    pub pawn: Pawn,
    pub weapon: CurrentWeapon,
    pub position: Position,
    // Pawn entities must be saved to disk:
    pub save: Save,
}

impl PawnBundle {
    fn new(weapon: Entity, position: Vec2) -> Self {
        Self {
            pawn: Pawn,
            weapon: CurrentWeapon(Some(weapon)),
            position: Position(position),
            save: Save,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Weapon;

#[derive(Bundle, Default)]
struct WeaponBundle {
    // Weapon data:
    pub weapon: Weapon,
    /* ... */
    // Weapon entities must be saved to disk:
    pub save: Save,
}

#[derive(Component)]
struct Sprite(Entity);

#[derive(Bundle, Default)]
struct PawnSpriteBundle {
    // Pawn visuals (i.e. scene components like SpriteBundle, TransformBundle, Parent, Children, etc.):
    /* ... */
    // Pawn sprite entities are never saved to disk, and must be reconstructed
    // from loaded data, so unload them prior to load from disk:
    pub unload: Unload,
}

// Setup the world:
fn setup(mut commands: Commands) {
    // Since the Camera entity is always spawned unconditionally, it does not need to be unloaded or saved.
    commands.spawn(Camera2dBundle::default());
}

// Spawn a new pawn sprite for every spawned pawn.
fn spawn_pawn_sprites(query: Query<Entity, Added<Pawn>>, mut commands: Commands) {
    for entity in &query {
        let model_entity = commands.spawn(PawnSpriteBundle::default()).id();
        commands.entity(entity).insert(Sprite(model_entity));
    }
}

// Update the position of pawn sprites:
// Note: This system isn't required for this example, but it illustrates how to update the transform from the saved position.
fn update_model_position(
    query: Query<(&Position, &Sprite), Changed<Position>>,
    mut model_query: Query<&mut Transform>,
) {
    for (&Position(xy), &Sprite(model_entity)) in &query {
        model_query.get_mut(model_entity).unwrap().translation = xy.extend(0.0);
    }
}

// System to fix up weapon references:
fn post_load(
    mut query: Query<&mut CurrentWeapon>,
    loaded_weapons: Query<(Entity, &Loaded), With<Weapon>>,
) {
    // First, construct a mapping of all loaded indices to their new entities.
    // Notice how `Loaded` does not return an `Entity`. This is because the entity's generation
    // is invalidated upon load. So only the entity index may be used for post-load fix up.
    let loaded_weapons: HashMap<u32, Entity> = loaded_weapons
        .iter()
        .map(|(entity, loaded)| (loaded.index(), entity))
        .collect();
    // Then update each `CurrentWeapon` reference using the new entity mapping:
    for mut current_weapon in &mut query {
        if let Some(old_entity) = current_weapon.entity() {
            if let Some(new_entity) = loaded_weapons.get(&old_entity.index()) {
                *current_weapon = CurrentWeapon(Some(*new_entity));
            }
        }
    }
}
