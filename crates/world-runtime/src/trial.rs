use crate::{RuntimeError, RuntimeWorld, SaveStore, Snapshot, StoredWorld, WorldRuntime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_TRIAL_STEPS: u32 = 1200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialForkBase {
    pub world_id: String,
    pub run_generation: String,
    pub edit_revision: String,
    pub simulation_tick: String,
}

impl TrialForkBase {
    fn from_world(world: &RuntimeWorld) -> Self {
        Self {
            world_id: world.saved.world_id.clone(),
            run_generation: world.generation.clone(),
            edit_revision: world.saved.edit_revision.to_string(),
            simulation_tick: world.saved.state.tick().to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum TrialRate {
    #[serde(rename = "1x")]
    Normal = 1,
    #[serde(rename = "10x")]
    Tenfold = 10,
    #[serde(rename = "60x")]
    Sixtyfold = 60,
}

impl WorldRuntime {
    pub fn start_trial(&mut self, source_id: &str) -> Result<Snapshot, RuntimeError> {
        if self.suspended {
            return Err(RuntimeError::Suspended);
        }
        self.require_formal(source_id)?;
        let source = self.world(source_id)?;
        if !source.saved.is_paused() {
            return Err(RuntimeError::EditRequiresPause);
        }

        let fork_base = TrialForkBase::from_world(source);
        let id = format!("trial-{}", Uuid::new_v4().simple());
        let mut saved = source.saved.clone();
        saved.world_id = id.clone();
        // A workshop trial starts at the sampled phase, then follows its own
        // simulated day. It does not chase the changing Windows clock.
        saved.state.clear_daylight_target();
        let trial = RuntimeWorld::new(saved);
        let snapshot = trial.snapshot();
        self.trials.insert(id.clone(), fork_base);
        self.worlds.insert(id, trial);
        Ok(snapshot)
    }

    pub fn trial_fork_base(&self, id: &str) -> Result<TrialForkBase, RuntimeError> {
        self.trials.get(id).cloned().ok_or(RuntimeError::TrialMissing)
    }

    pub fn advance_trial(&mut self, id: &str, ticks: u32) -> Result<Snapshot, RuntimeError> {
        if !(1..=MAX_TRIAL_STEPS).contains(&ticks) {
            return Err(RuntimeError::InvalidTrialTicks);
        }
        if self.suspended {
            return Err(RuntimeError::Suspended);
        }
        if !self.trials.contains_key(id) {
            return Err(RuntimeError::TrialMissing);
        }
        let trial = self.world_mut(id)?;
        trial.advance(ticks)?;
        Ok(trial.snapshot())
    }

    /// The caller owns elapsed awake time. A single call remains bounded even
    /// if the selected multiplier or supplied elapsed time is large.
    pub fn advance_trial_scaled(&mut self, id: &str, elapsed_ticks: u32, rate: TrialRate) -> Result<Snapshot, RuntimeError> {
        let ticks = elapsed_ticks.checked_mul(rate as u32).ok_or(RuntimeError::InvalidTrialTicks)?;
        self.advance_trial(id, ticks)
    }

    pub fn discard_trial(&mut self, id: &str) -> Result<(), RuntimeError> {
        if self.trials.remove(id).is_none() {
            return Err(RuntimeError::TrialMissing);
        }
        self.worlds.remove(id);
        Ok(())
    }

    /// Save a complete formal checkpoint before replacing the authoritative
    /// world. A conflict or write failure leaves the source and trial intact.
    pub fn apply_trial(&mut self, store: &SaveStore, id: &str) -> Result<Snapshot, RuntimeError> {
        if self.suspended {
            return Err(RuntimeError::Suspended);
        }
        let fork_base = self.trial_fork_base(id)?;
        let source = self.worlds.get(&fork_base.world_id).ok_or(RuntimeError::TrialConflict)?;
        let trial = self.world(id)?;
        if fork_base != TrialForkBase::from_world(source)
            || trial.saved.state.model_version() != source.saved.state.model_version()
            || trial.saved.state.definition() != source.saved.state.definition()
        {
            return Err(RuntimeError::TrialConflict);
        }

        let mut saved = source.saved.clone();
        saved.edit_revision = saved.edit_revision.checked_add(1).ok_or(RuntimeError::RevisionOverflow)?;
        saved.state = trial.saved.state.clone();
        saved.observation_history = trial.saved.observation_history.clone();
        saved.state.clear_daylight_target();
        if let Some(target) = source.saved.state.daylight_target() {
            saved.state.set_daylight_target(target)?;
        }

        let adopted = RuntimeWorld::new(saved);
        store.save(&adopted.saved)?;
        let snapshot = adopted.snapshot();
        self.worlds.insert(fork_base.world_id, adopted);
        self.trials.remove(id);
        self.worlds.remove(id);
        Ok(snapshot)
    }

    /// The trial remains in memory until the caller finishes publishing its
    /// new world record and explicitly discards the temporary branch.
    pub fn save_trial_as_world(&mut self, store: &SaveStore, id: &str) -> Result<Snapshot, RuntimeError> {
        if !self.trials.contains_key(id) {
            return Err(RuntimeError::TrialMissing);
        }
        let saved = self.world(id)?.saved.clone();
        self.save_as_new_world(store, saved)
    }

    pub fn copy_world(&mut self, store: &SaveStore, source_id: &str) -> Result<Snapshot, RuntimeError> {
        self.require_formal(source_id)?;
        let saved = self.world(source_id)?.saved.clone();
        self.save_as_new_world(store, saved)
    }

    fn save_as_new_world(&mut self, store: &SaveStore, mut saved: StoredWorld) -> Result<Snapshot, RuntimeError> {
        if self.suspended {
            return Err(RuntimeError::Suspended);
        }
        let id = format!("world-{}", Uuid::new_v4().simple());
        saved.world_id = id.clone();
        saved.edit_revision = 0;
        saved.paused = true;
        saved.edit_paused = false;
        let world = RuntimeWorld::new(saved);
        store.save(&world.saved)?;
        let snapshot = world.snapshot();
        self.worlds.insert(id, world);
        Ok(snapshot)
    }

    pub(crate) fn require_formal(&self, id: &str) -> Result<(), RuntimeError> {
        if self.trials.contains_key(id) {
            return Err(RuntimeError::TrialOperationInvalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PondEdit, SolarLocation};

    fn store_for_test(label: &str) -> (std::path::PathBuf, SaveStore) {
        let path = std::env::temp_dir().join(format!("mico-m3-{label}-{}", Uuid::new_v4()));
        let store = SaveStore::new(&path);
        (path, store)
    }

    #[test]
    fn trial_drives_the_whole_model_and_daylight_with_reproducible_scaled_steps() {
        let mut runtime = WorldRuntime::new();
        runtime.create_pond("pond", 2729).unwrap();
        runtime.set_daylight_target("pond", 0.99).unwrap();
        let formal = runtime.set_edit_paused("pond", true).unwrap();
        let fast = runtime.start_trial("pond").unwrap();
        let control = runtime.start_trial("pond").unwrap();
        assert_eq!(fast.daylight_phase, formal.daylight_phase);
        let base = runtime.trial_fork_base(&fast.world_id).unwrap();
        assert_eq!(base.world_id, formal.world_id);
        assert_eq!(base.run_generation, formal.run_generation);
        assert_eq!(base.edit_revision, formal.edit_revision);
        assert_eq!(base.simulation_tick, formal.simulation_tick);
        assert_eq!(runtime.world(&fast.world_id).unwrap().saved.state.daylight_target(), None);
        assert_eq!(runtime.set_daylight_target(&fast.world_id, 0.01), Err(RuntimeError::TrialOperationInvalid));
        assert_eq!(runtime.advance_ticks(&fast.world_id, 1), Err(RuntimeError::TrialOperationInvalid));
        assert_eq!(runtime.advance_trial_scaled(&fast.world_id, 21, TrialRate::Sixtyfold), Err(RuntimeError::InvalidTrialTicks));
        assert_eq!(runtime.advance_trial_scaled(&fast.world_id, u32::MAX, TrialRate::Tenfold), Err(RuntimeError::InvalidTrialTicks));
        let advanced = runtime.advance_trial_scaled(&fast.world_id, 10, TrialRate::Sixtyfold).unwrap();
        for _ in 0..20 {
            runtime.advance_trial_scaled(&control.world_id, 3, TrialRate::Tenfold).unwrap();
        }
        assert_eq!(advanced.simulation_tick, "600");
        assert_ne!(advanced.pond, formal.pond);
        assert_eq!(runtime.world(&fast.world_id).unwrap().saved.state, runtime.world(&control.world_id).unwrap().saved.state);
        assert!((advanced.daylight_phase - formal.daylight_phase - 60.0 / 86400.0).abs() < 1e-9);
        assert_eq!(runtime.snapshot("pond").unwrap(), formal);
    }

    #[test]
    fn apply_is_durable_and_keeps_formal_identity_pause_location_and_live_goal() {
        let (path, store) = store_for_test("adopt");
        let mut runtime = WorldRuntime::new();
        runtime.create_pond("pond", 2729).unwrap();
        runtime.set_paused("pond", true).unwrap();
        let blank = runtime.set_edit_paused("pond", true).unwrap();
        let source = runtime.edit_pond_command("pond", "shape", &blank.run_generation, "0", PondEdit::Reshape {
            basin_id: "pond-main".to_owned(), surface_area_cm2: 2200.0,
        }).unwrap();
        runtime.save(&store, "pond").unwrap();
        let fork = runtime.start_trial("pond").unwrap();
        let branch = runtime.advance_trial(&fork.world_id, 600).unwrap();
        let location = SolarLocation { name: "成都".to_owned(), latitude_degrees: 30.57, longitude_degrees: 104.07 };
        runtime.set_location("pond", location.clone()).unwrap();
        runtime.set_daylight_target("pond", 0.26).unwrap();

        let mut expected_state = runtime.world(&fork.world_id).unwrap().saved.state.clone();
        expected_state.set_daylight_target(0.26).unwrap();
        let adopted = runtime.apply_trial(&store, &fork.world_id).unwrap();
        assert_eq!(adopted.world_id, source.world_id);
        assert_ne!(adopted.run_generation, source.run_generation);
        assert_ne!(adopted.run_generation, fork.run_generation);
        assert_eq!(adopted.edit_revision, "2");
        assert_eq!(adopted.simulation_tick, branch.simulation_tick);
        assert_eq!(adopted.pond, branch.pond);
        assert_eq!(adopted.daylight_phase, branch.daylight_phase);
        assert_eq!(adopted.location, Some(location));
        assert!(adopted.paused && adopted.manual_paused && adopted.edit_paused);
        assert!(!adopted.pond_undo_available);
        assert_eq!(runtime.world(&adopted.world_id).unwrap().saved.state, expected_state);
        assert_eq!(store.load("pond").unwrap().world, runtime.world("pond").unwrap().saved);
        assert_eq!(runtime.trial_fork_base(&fork.world_id), Err(RuntimeError::TrialMissing));
        assert!(matches!(runtime.edit_pond_history_command("pond", "old", &source.run_generation, "1", false), Err(RuntimeError::StaleGeneration { .. })));
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn revision_steps_and_generation_each_block_adoption_without_losing_the_trial() {
        let (path, store) = store_for_test("conflict");
        let mut runtime = WorldRuntime::new();
        let first = runtime.create_forest("forest", 1206).unwrap();
        runtime.set_edit_paused("forest", true).unwrap();
        runtime.save(&store, "forest").unwrap();
        let revision_trial = runtime.start_trial("forest").unwrap();
        let moved = runtime.edit_rock("forest", &first.run_generation, "0", 0.74, 0.65).unwrap();
        assert_eq!(runtime.apply_trial(&store, &revision_trial.world_id), Err(RuntimeError::TrialConflict));
        assert_eq!(runtime.snapshot("forest").unwrap(), moved);
        assert_eq!(runtime.snapshot(&revision_trial.world_id).unwrap(), revision_trial);

        let tick_trial = runtime.start_trial("forest").unwrap();
        runtime.set_edit_paused("forest", false).unwrap();
        runtime.advance_ticks("forest", 1).unwrap();
        let advanced = runtime.set_edit_paused("forest", true).unwrap();
        assert_eq!(runtime.apply_trial(&store, &tick_trial.world_id), Err(RuntimeError::TrialConflict));
        assert_eq!(runtime.snapshot("forest").unwrap(), advanced);
        assert_eq!(runtime.snapshot(&tick_trial.world_id).unwrap(), tick_trial);

        let generation_trial = runtime.start_trial("forest").unwrap();
        runtime.save(&store, "forest").unwrap();
        let loaded = runtime.load(&store, "forest").unwrap().snapshot;
        assert_ne!(loaded.run_generation, advanced.run_generation);
        assert_eq!(loaded.simulation_tick, advanced.simulation_tick);
        assert_eq!(loaded.edit_revision, advanced.edit_revision);
        assert_eq!(runtime.apply_trial(&store, &generation_trial.world_id), Err(RuntimeError::TrialConflict));
        assert_eq!(runtime.snapshot("forest").unwrap(), loaded);
        assert_eq!(runtime.snapshot(&generation_trial.world_id).unwrap(), generation_trial);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn failed_checkpoint_keeps_both_branches_and_allows_retry() {
        let (path, good_store) = store_for_test("retry");
        std::fs::create_dir_all(&path).unwrap();
        let blocked = path.join("file-instead-of-directory");
        std::fs::write(&blocked, "blocked").unwrap();
        let bad_store = SaveStore::new(blocked);
        let mut runtime = WorldRuntime::new();
        runtime.create_pond("pond", 2729).unwrap();
        let source = runtime.set_edit_paused("pond", true).unwrap();
        let fork = runtime.start_trial("pond").unwrap();
        let trial = runtime.advance_trial(&fork.world_id, 600).unwrap();
        assert!(matches!(runtime.apply_trial(&bad_store, &fork.world_id), Err(RuntimeError::CheckpointIo { .. })));
        assert_eq!(runtime.snapshot("pond").unwrap(), source);
        assert_eq!(runtime.snapshot(&fork.world_id).unwrap(), trial);
        assert!(matches!(runtime.save_trial_as_world(&bad_store, &fork.world_id), Err(RuntimeError::CheckpointIo { .. })));
        assert_eq!(runtime.snapshot(&fork.world_id).unwrap(), trial);
        assert_eq!(runtime.apply_trial(&good_store, &fork.world_id).unwrap().simulation_tick, trial.simulation_tick);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn copy_and_trial_save_commit_distinct_exact_states_without_changing_original() {
        let (path, store) = store_for_test("branch");
        let mut runtime = WorldRuntime::new();
        runtime.create_pond("pond", 2729).unwrap();
        runtime.set_daylight_target("pond", 0.25).unwrap();
        runtime.advance_ticks("pond", 40).unwrap();
        let source = runtime.set_edit_paused("pond", true).unwrap();
        let fork = runtime.start_trial("pond").unwrap();
        let trial = runtime.advance_trial(&fork.world_id, 600).unwrap();
        let copy = runtime.copy_world(&store, "pond").unwrap();
        let saved_trial = runtime.save_trial_as_world(&store, &fork.world_id).unwrap();
        assert_eq!(runtime.snapshot("pond").unwrap(), source);
        assert_eq!(runtime.snapshot(&fork.world_id).unwrap(), trial);
        assert_ne!(copy.world_id, source.world_id);
        assert_ne!(saved_trial.world_id, trial.world_id);
        assert_ne!(copy.world_id, saved_trial.world_id);
        assert_eq!(copy.simulation_tick, source.simulation_tick);
        assert_eq!(saved_trial.simulation_tick, trial.simulation_tick);
        assert!(copy.manual_paused && !copy.edit_paused);
        assert!(saved_trial.manual_paused && !saved_trial.edit_paused);
        assert_eq!(store.load(&copy.world_id).unwrap().world.state, runtime.world("pond").unwrap().saved.state);
        assert_eq!(store.load(&saved_trial.world_id).unwrap().world.state, runtime.world(&fork.world_id).unwrap().saved.state);
        assert!(matches!(store.load(&fork.world_id), Err(RuntimeError::CheckpointMissing { .. })));
        std::fs::remove_dir_all(path).unwrap();
    }
}
