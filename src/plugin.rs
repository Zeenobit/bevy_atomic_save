use super::*;

/// A [`Plugin`] which adds the [`SaveStage`] and any required systems for saving and loading the [`World`].
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_stage_after(
            CoreStage::Last,
            SaveStage::Save,
            SystemStage::single(save).with_run_criteria(should_save),
        )
        .add_stage_before(
            CoreStage::PreUpdate,
            SaveStage::Load,
            SystemStage::single(load).with_run_criteria(should_load),
        )
        .add_stage_after(
            SaveStage::Load,
            SaveStage::PostLoad,
            SystemStage::parallel().with_run_criteria(should_load),
        )
        .add_system_to_stage(SaveStage::PostLoad, finish_load);
    }
}
