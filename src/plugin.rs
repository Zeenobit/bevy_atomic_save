use super::*;

/// A [`Plugin`] which adds the [`SaveSet`] and any required systems for saving and loading the [`World`].
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(SaveSet::Save.after(CoreSet::Last))
            .add_system(save.in_base_set(SaveSet::Save).run_if(should_save))
            .configure_sets((CoreSet::PreUpdate, SaveSet::Load, SaveSet::PostLoad).chain())
            .add_system(load.in_base_set(SaveSet::Load).run_if(should_load))
            .add_system(
                finish_load
                    .in_base_set(SaveSet::PostLoad)
                    .run_if(should_load),
            );
    }
}
