//! A portable backup contains one durable world and its display name.

use super::checkpoint::{
    checkpoint_path, fits_first_checkpoint, fnv1a_checksum, read_world_payload, ReadError, MAX_CHECKPOINT_BYTES,
};
use super::library::ENTRY_FILENAME;
use super::{validate_world_id, RuntimeError, SaveStore, StoredWorld, WorldRuntime};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use uuid::Uuid;

const MAGIC: &str = "micro-ecosystem-world-backup";
const LEGACY_FORMAT_VERSION: u32 = 1;
const WOOD_BIOFILM_FORMAT_VERSION: u32 = 2;
// Room for the envelope and a UTF-8 display name in addition to a checkpoint.
pub const MAX_WORLD_BACKUP_BYTES: usize = MAX_CHECKPOINT_BYTES as usize + 2 * 1024;
const PORTABLE_WORLD_ID: &str = "world-00000000000000000000000000000000";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldBackupImport {
    pub world_id: String,
    pub name: String,
    pub model_version: u32,
    pub definition: String,
    pub simulation_tick: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupHeader {
    magic: String,
    format_version: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BackupEnvelope {
    magic: String,
    format_version: u32,
    name: String,
    checksum: String,
    payload: String,
}

impl WorldRuntime {
    /// Use the current authoritative state, or the last readable checkpoint for
    /// an unloaded world. Exporting does not save, resume or switch worlds.
    pub fn export_world_backup(&self, store: &SaveStore, world_id: &str, name: &str) -> Result<String, RuntimeError> {
        validate_world_id(world_id)?;
        self.require_formal(world_id)?;
        let name = SaveStore::validate_library_name(name)?;
        let mut saved = match self.worlds.get(world_id) {
            Some(world) => world.saved.clone(),
            None => store.load(world_id)?.world,
        };
        saved.world_id = PORTABLE_WORLD_ID.to_owned();
        let payload = serde_json::to_string(&saved).map_err(|_| RuntimeError::WorldBackupInvalid)?;
        prepare_copy(&mut saved);
        check_copy_size(&saved)?;
        let envelope = BackupEnvelope {
            magic: MAGIC.to_owned(),
            format_version: if saved.state.has_wood_biofilm() { WOOD_BIOFILM_FORMAT_VERSION } else { LEGACY_FORMAT_VERSION },
            checksum: backup_checksum(&name, &payload),
            name,
            payload,
        };
        let contents = serde_json::to_string(&envelope).map_err(|_| RuntimeError::WorldBackupInvalid)?;
        if contents.len() > MAX_WORLD_BACKUP_BYTES { return Err(RuntimeError::WorldBackupTooLarge); }
        Ok(contents)
    }
}

impl SaveStore {
    /// Publish a complete independent copy. Loading it into a view is a
    /// separate action and the active selection is never modified here.
    pub fn import_world_backup(&self, contents: &str) -> Result<WorldBackupImport, RuntimeError> {
        if contents.len() > MAX_WORLD_BACKUP_BYTES { return Err(RuntimeError::WorldBackupTooLarge); }
        let header: BackupHeader = serde_json::from_str(contents).map_err(|_| RuntimeError::WorldBackupInvalid)?;
        if header.magic != MAGIC { return Err(RuntimeError::WorldBackupInvalid); }
        if !matches!(header.format_version, LEGACY_FORMAT_VERSION | WOOD_BIOFILM_FORMAT_VERSION) {
            return Err(RuntimeError::WorldBackupUnsupported);
        }
        let envelope: BackupEnvelope = serde_json::from_str(contents).map_err(|_| RuntimeError::WorldBackupInvalid)?;
        let name = Self::validate_library_name(&envelope.name).map_err(|_| RuntimeError::WorldBackupInvalid)?;
        if name != envelope.name || envelope.checksum != backup_checksum(&name, &envelope.payload) {
            return Err(RuntimeError::WorldBackupInvalid);
        }
        let mut saved = read_world_payload(&envelope.payload, None).map_err(|error| match error {
            ReadError::Unsupported(_) => RuntimeError::WorldBackupUnsupported,
            _ => RuntimeError::WorldBackupInvalid,
        })?;
        if saved.state.has_wood_biofilm() != (envelope.format_version == WOOD_BIOFILM_FORMAT_VERSION) {
            return Err(RuntimeError::WorldBackupInvalid);
        }
        saved.world_id = format!("world-{}", Uuid::new_v4().simple());
        prepare_copy(&mut saved);
        check_copy_size(&saved)?;
        self.publish_backup_world(&saved, &name)?;
        Ok(WorldBackupImport {
            world_id: saved.world_id,
            name,
            model_version: saved.state.model_version(),
            definition: saved.state.definition().to_owned(),
            simulation_tick: saved.state.tick().to_string(),
        })
    }

    fn publish_backup_world(&self, saved: &StoredWorld, name: &str) -> Result<(), RuntimeError> {
        fs::create_dir_all(&self.root).map_err(|error| backup_io("建立世界库目录", error))?;
        let target = self.world_directory(&saved.world_id);
        match fs::symlink_metadata(&target) {
            Ok(_) => return Err(RuntimeError::WorldBackupIo { details: "无法为备份分配独立的世界目录。请重试。".to_owned() }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(backup_io("检查导入目标", error)),
        }
        // Discovery ignores staging folders. Persist the checkpoint and its
        // library record in that folder before publishing the world directory.
        let staging = SaveStore::new(self.root.join(format!(".world-import-{}", Uuid::new_v4().simple())));
        fs::create_dir(&staging.root).map_err(|error| backup_io("建立导入临时目录", error))?;
        let staged_world = staging.world_directory(&saved.world_id);
        let published = (|| {
            staging.save(saved).map_err(|_| RuntimeError::WorldBackupIo { details: "无法写入备份的初始检查点。".to_owned() })?;
            staging.set_world_name(&saved.world_id, name)
                .map_err(|_| RuntimeError::WorldBackupIo { details: "无法写入备份的世界名称。".to_owned() })?;
            fs::rename(&staged_world, target).map_err(|error| backup_io("提交备份副本", error))?;
            Ok(())
        })();
        if published.is_err() {
            let _ = fs::remove_file(staged_world.join(ENTRY_FILENAME));
            let _ = fs::remove_file(checkpoint_path(&staged_world, 1));
            let _ = fs::remove_dir(staged_world);
        }
        let _ = fs::remove_dir(staging.root);
        published
    }
}

fn prepare_copy(saved: &mut StoredWorld) {
    saved.edit_revision = 0;
    saved.paused = true;
    saved.edit_paused = false;
}

fn check_copy_size(saved: &StoredWorld) -> Result<(), RuntimeError> {
    match fits_first_checkpoint(saved) {
        Ok(true) => Ok(()),
        Ok(false) => Err(RuntimeError::WorldBackupTooLarge),
        Err(_) => Err(RuntimeError::WorldBackupInvalid),
    }
}

fn backup_checksum(name: &str, payload: &str) -> String {
    fnv1a_checksum(name.bytes().chain([0]).chain(payload.bytes()))
}

fn backup_io(operation: &str, error: io::Error) -> RuntimeError {
    let reason = match error.kind() {
        io::ErrorKind::PermissionDenied => "没有写入权限",
        io::ErrorKind::StorageFull => "磁盘空间不足",
        _ => "无法写入文件",
    };
    RuntimeError::WorldBackupIo { details: format!("{operation}失败。{reason}。") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solar::SolarLocation;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    struct TestStore {
        path: PathBuf,
        parent: PathBuf,
    }

    impl TestStore {
        fn new() -> Self {
            let parent = std::env::temp_dir().canonicalize().unwrap();
            let path = parent.join(format!("micro-ecosystem-backup-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self { path, parent }
        }

        fn store(&self) -> SaveStore { SaveStore::new(&self.path) }
    }

    impl Drop for TestStore {
        fn drop(&mut self) {
            if let Ok(path) = self.path.canonicalize() {
                if path.parent() == Some(&self.parent) {
                    let _ = fs::remove_dir_all(path);
                }
            }
        }
    }

    #[test]
    fn backup_replays_the_exact_forest_and_pond_state_without_changing_originals() {
        for (preset, seed) in [("forest-r5-composed", 1206), ("pond-r5-composed", 2729)] {
            let directory = TestStore::new();
            let store = directory.store();
            let mut runtime = WorldRuntime::new();
            let original = runtime.create_library_world(&store, preset, seed).unwrap();
            let source_id = original.world_id;
            runtime.save(&store, &source_id).unwrap();
            store.set_world_name(&source_id, "原世界").unwrap();
            store.select_active_world(&source_id).unwrap();
            let source_path = checkpoint_path(&store.world_directory(&source_id), 1);
            let source_bytes = fs::read(&source_path).unwrap();
            let selection = fs::read(store.root.join("active-world.json")).unwrap();

            runtime.set_location(&source_id, SolarLocation {
                latitude_degrees: 28.23, longitude_degrees: 112.94, name: "长沙".into(),
            }).unwrap();
            runtime.set_daylight_target(&source_id, 0.81).unwrap();
            runtime.set_edit_paused(&source_id, false).unwrap();
            runtime.set_paused(&source_id, false).unwrap();
            runtime.advance_ticks(&source_id, 257).unwrap();
            let original = runtime.world(&source_id).unwrap().saved.clone();
            assert!(!original.observation_history.is_empty());

            let contents = runtime.export_world_backup(&store, &source_id, "原世界").unwrap();
            assert!(!contents.contains(&source_id));
            let imported = store.import_world_backup(&contents).unwrap();
            let duplicate = store.import_world_backup(&contents).unwrap();
            assert_ne!(imported.world_id, source_id);
            assert_ne!(imported.world_id, duplicate.world_id);
            assert_eq!(imported.name, duplicate.name);
            assert_eq!(imported.simulation_tick, "257");
            assert_eq!(imported.definition, original.state.definition());
            assert_eq!(imported.model_version, original.state.model_version());
            assert_eq!(store.discover_worlds().unwrap().len(), 3);
            assert_eq!(fs::read(&source_path).unwrap(), source_bytes);
            assert_eq!(fs::read(store.root.join("active-world.json")).unwrap(), selection);
            assert_eq!(runtime.world(&source_id).unwrap().saved, original);
            assert!(!runtime.is_world_loaded(&imported.world_id));

            let mut reopened = WorldRuntime::new();
            let snapshot = reopened.load(&store, &imported.world_id).unwrap().snapshot;
            assert!(snapshot.manual_paused);
            assert!(!snapshot.edit_paused);
            assert_eq!(snapshot.edit_revision, "0");
            let restored = &reopened.world(&imported.world_id).unwrap().saved;
            assert_eq!(restored.state, original.state);
            assert_eq!(restored.solar_config, original.solar_config);
            assert_eq!(restored.observation_history, original.observation_history);

            reopened.set_paused(&imported.world_id, false).unwrap();
            reopened.advance_ticks(&imported.world_id, 71).unwrap();
            runtime.advance_ticks(&source_id, 71).unwrap();
            let continued = &reopened.world(&imported.world_id).unwrap().saved;
            let expected = &runtime.world(&source_id).unwrap().saved;
            assert_eq!(continued.state, expected.state);
            assert_eq!(continued.observation_history, expected.observation_history);
            assert_eq!(store.active_world().unwrap().as_deref(), Some(source_id.as_str()));
        }
    }

    #[test]
    fn invalid_backup_variants_publish_no_world_or_selection_changes() {
        let directory = TestStore::new();
        let store = directory.store();
        let mut runtime = WorldRuntime::new();
        runtime.create_composed_forest("existing", 1206).unwrap();
        runtime.save(&store, "existing").unwrap();
        store.select_active_world("existing").unwrap();
        let valid = runtime.export_world_backup(&store, "existing", "原世界").unwrap();
        let edit_envelope = |change: fn(&mut BackupEnvelope)| {
            let mut envelope: BackupEnvelope = serde_json::from_str(&valid).unwrap();
            change(&mut envelope);
            serde_json::to_string(&envelope).unwrap()
        };
        let change_model = |envelope: &mut BackupEnvelope| {
            let mut payload: Value = serde_json::from_str(&envelope.payload).unwrap();
            payload["state"]["fixtureDefinition"] = json!("unsupported/1");
            envelope.payload = serde_json::to_string(&payload).unwrap();
            envelope.checksum = backup_checksum(&envelope.name, &envelope.payload);
        };
        let change_solar = |envelope: &mut BackupEnvelope| {
            let mut payload: Value = serde_json::from_str(&envelope.payload).unwrap();
            payload["solarConfig"]["location"]["latitudeDegrees"] = json!(91);
            envelope.payload = serde_json::to_string(&payload).unwrap();
            envelope.checksum = backup_checksum(&envelope.name, &envelope.payload);
        };
        let before = store.discover_worlds().unwrap();
        for (input, error) in [
            ("{".into(), RuntimeError::WorldBackupInvalid),
            (edit_envelope(|envelope| envelope.name = "different".into()), RuntimeError::WorldBackupInvalid),
            (edit_envelope(|envelope| envelope.payload.push(' ')), RuntimeError::WorldBackupInvalid),
            (edit_envelope(|envelope| envelope.format_version = WOOD_BIOFILM_FORMAT_VERSION + 1), RuntimeError::WorldBackupUnsupported),
            (edit_envelope(change_model), RuntimeError::WorldBackupUnsupported),
            (edit_envelope(change_solar), RuntimeError::WorldBackupInvalid),
            (" ".repeat(MAX_WORLD_BACKUP_BYTES + 1), RuntimeError::WorldBackupTooLarge),
        ] {
            assert_eq!(store.import_world_backup(&input), Err(error));
            assert_eq!(store.discover_worlds().unwrap(), before);
            assert_eq!(store.active_world().unwrap().as_deref(), Some("existing"));
        }
        assert_eq!(fs::read_dir(&store.root).unwrap().count(), 2);
    }

    #[test]
    fn backup_version_gates_wood_biofilm_without_rejecting_old_backups() {
        let directory = TestStore::new();
        let store = directory.store();
        let mut runtime = WorldRuntime::new();
        runtime.create_pond("older-pond", 2729).unwrap();
        let current = runtime.create_pond_from_preset(2729).unwrap();
        let old = runtime.export_world_backup(&store, "older-pond", "旧浅塘").unwrap();
        let new = runtime.export_world_backup(&store, &current.world_id, "沉木库存").unwrap();
        let legacy: BackupHeader = serde_json::from_str(&old).unwrap();
        let mut modern: BackupEnvelope = serde_json::from_str(&new).unwrap();
        assert_eq!(legacy.format_version, LEGACY_FORMAT_VERSION);
        assert_eq!(modern.format_version, WOOD_BIOFILM_FORMAT_VERSION);

        let old_copy = store.import_world_backup(&old).unwrap();
        let new_copy = store.import_world_backup(&new).unwrap();
        assert!(!store.load(&old_copy.world_id).ok().unwrap().world.state.has_wood_biofilm());
        assert_eq!(store.load(&new_copy.world_id).ok().unwrap().world.state,
            runtime.world(&current.world_id).unwrap().saved.state);

        modern.format_version = LEGACY_FORMAT_VERSION;
        assert_eq!(store.import_world_backup(&serde_json::to_string(&modern).unwrap()), Err(RuntimeError::WorldBackupInvalid));
        assert_eq!(store.discover_worlds().unwrap().len(), 2);
    }

    #[test]
    fn export_can_use_an_unloaded_checkpoint_but_never_a_trial() {
        let directory = TestStore::new();
        let store = directory.store();
        let mut runtime = WorldRuntime::new();
        runtime.create_seasonal_forest("original", 719_521).unwrap();
        runtime.advance_ticks("original", 19).unwrap();
        runtime.set_edit_paused("original", true).unwrap();
        runtime.save(&store, "original").unwrap();
        let trial = runtime.start_trial("original").unwrap();
        assert_eq!(runtime.export_world_backup(&store, &trial.world_id, "预览"), Err(RuntimeError::TrialOperationInvalid));
        let from_live = runtime.export_world_backup(&store, "original", "原世界").unwrap();
        runtime.unload("original");
        assert_eq!(runtime.export_world_backup(&store, "original", "原世界").unwrap(), from_live);
        let copy = store.import_world_backup(&from_live).unwrap();
        assert_eq!(copy.model_version, eco_core::FOREST_SEASONAL_MODEL_VERSION);
        assert_eq!(copy.simulation_tick, "19");
        assert!(!runtime.is_world_loaded("original"));
    }
}
