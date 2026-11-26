use super::{validate_world_id, RuntimeError, StoredWorld};
use super::solar::SOLAR_MODEL_VERSION;
use eco_core::{CoreError, WorldState};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAGIC: &str = "micro-ecosystem-fixture";
const LEGACY_FORMAT_VERSION: u32 = 1;
const FORMAT_VERSION: u32 = 2;
const MAX_CHECKPOINT_BYTES: u64 = 256 * 1024;
const AUTOMATIC_HISTORY: usize = 2;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReceipt {
    pub world_id: String,
    pub run_generation: String,
    pub simulation_tick: String,
    pub checkpoint_serial: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    magic: String,
    format_version: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    magic: String,
    format_version: u32,
    checkpoint_serial: u64,
    checksum: String,
    payload: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayloadHeader {
    state: ModelIdentity,
    solar_config: Option<SolarIdentity>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelIdentity {
    model_version: u32,
    fixture_definition: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolarIdentity {
    model_version: u16,
}

pub(super) struct LoadedCheckpoint {
    pub world: StoredWorld,
    pub serial: u64,
    pub recovered_previous: bool,
    pub automatic: bool,
}

enum ReadError {
    Corrupt(String),
    Unsupported(String),
    Io(RuntimeError),
}

/// Stores immutable checkpoints. Exactly one application runtime should own a storage root.
/// Explicit checkpoints and unusable files are retained for recovery and inspection.
#[derive(Clone, Debug)]
pub struct SaveStore {
    pub(super) root: PathBuf,
}

impl SaveStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn active_world(&self) -> Result<Option<String>, RuntimeError> {
        let path = self.root.join("active-world.json");
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_error("read active world", &path, error)),
        };
        let mut bytes = Vec::new();
        file.take(128).read_to_end(&mut bytes).map_err(|error| io_error("read active world", &path, error))?;
        let id: String = serde_json::from_slice(&bytes).map_err(|error| serialization_error("read active world", error))?;
        validate_world_id(&id)?;
        Ok(Some(id))
    }

    pub fn select_active_world(&self, world_id: &str) -> Result<(), RuntimeError> {
        // Only reference a usable, already committed world. Publication failure
        // preserves the previous selection and both world checkpoints.
        self.load(world_id)?;
        let path = self.root.join("active-world.json");
        let candidate = self.root.join(format!("active-{}.tmp", Uuid::new_v4()));
        let bytes = serde_json::to_vec(world_id).map_err(|error| serialization_error("serialize active world", error))?;
        let result = (|| -> io::Result<()> {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&candidate)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&candidate, &path)
        })();
        if result.is_err() { let _ = fs::remove_file(&candidate); }
        result.map_err(|error| io_error("publish active world", &path, error))
    }

    pub(super) fn save(&self, world: &StoredWorld) -> Result<u64, RuntimeError> {
        self.save_before_publish(world, false, || Ok(()))
    }

    pub(super) fn autosave(&self, world: &StoredWorld) -> Result<u64, RuntimeError> {
        self.save_before_publish(world, true, || Ok(()))
    }

    fn save_before_publish(
        &self,
        world: &StoredWorld,
        automatic: bool,
        before_publish: impl FnOnce() -> io::Result<()>,
    ) -> Result<u64, RuntimeError> {
        validate_world_id(&world.world_id)?;
        world.state.validate()?;
        if !world.observation_history.valid(&world.state) {
            return Err(RuntimeError::CheckpointIo { details: "观察记录与当前世界时间不一致。".to_owned() });
        }
        if world.solar_config.as_ref().is_some_and(|solar| !solar.location.validate()) {
            return Err(RuntimeError::InvalidLocation);
        }
        let directory = self.world_directory(&world.world_id);
        fs::create_dir_all(&directory).map_err(|err| io_error("create checkpoint directory", &directory, err))?;
        let previous = committed_checkpoints(&directory)?;
        let last = previous.last().map_or(0, |file| file.serial);
        let serial = last.checked_add(1).ok_or_else(|| RuntimeError::CheckpointIo {
            details: "checkpoint serial would overflow".to_owned(),
        })?;

        let payload = serde_json::to_string(world).map_err(|err| serialization_error("serialize fixture", err))?;
        let checkpoint = Envelope {
            magic: MAGIC.to_owned(),
            format_version: if world.observation_history.is_empty() { LEGACY_FORMAT_VERSION } else { FORMAT_VERSION },
            checkpoint_serial: serial,
            checksum: checkpoint_checksum(serial, payload.as_bytes()),
            payload,
        };
        let candidate = directory.join(format!("pending-{}.tmp", Uuid::new_v4()));
        let destination = CheckpointFile { serial, automatic }.path(&directory);

        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
            .map_err(|err| io_error("create checkpoint candidate", &candidate, err))?;
        let committed = publish_checkpoint(file, &candidate, &destination, &checkpoint, before_publish);
        // A retained candidate is ignored on every load. It cannot be mistaken for a commit.
        let _ = fs::remove_file(&candidate);
        committed?;
        if automatic {
            // The new file is durable. Keep at least one prior readable automatic
            // generation, and preserve any damaged or unfamiliar evidence.
            let mut readable = 1;
            for file in previous.into_iter().rev().filter(|file| file.automatic) {
                let path = file.path(&directory);
                if !fs::symlink_metadata(&path).is_ok_and(|entry| entry.file_type().is_file())
                    || read_checkpoint(&path, &world.world_id, file.serial).is_err() {
                    continue;
                }
                readable += 1;
                if readable > AUTOMATIC_HISTORY { let _ = fs::remove_file(&path); }
            }
        }
        Ok(serial)
    }

    pub(super) fn load(&self, world_id: &str) -> Result<LoadedCheckpoint, RuntimeError> {
        validate_world_id(world_id)?;
        let directory = self.world_directory(world_id);
        let serials = committed_checkpoints(&directory)?;
        if serials.is_empty() {
            return Err(RuntimeError::CheckpointMissing { world_id: world_id.to_owned() });
        }
        let mut newest_problem = None;
        for file in serials.into_iter().rev() {
            let path = file.path(&directory);
            match read_checkpoint(&path, world_id, file.serial) {
                Ok(world) => {
                    return Ok(LoadedCheckpoint { world, serial: file.serial, recovered_previous: newest_problem.is_some(), automatic: file.automatic });
                }
                Err(ReadError::Unsupported(details)) => {
                    return Err(RuntimeError::CheckpointUnsupported { world_id: world_id.to_owned(), details });
                }
                Err(ReadError::Io(error)) => return Err(error),
                Err(ReadError::Corrupt(details)) => {
                    newest_problem.get_or_insert(details);
                }
            }
        }
        Err(RuntimeError::CheckpointUnusable {
            world_id: world_id.to_owned(),
            details: newest_problem.expect("a committed checkpoint failed validation"),
        })
    }

    pub(super) fn world_directory(&self, world_id: &str) -> PathBuf {
        self.root.join(format!("world-{world_id}"))
    }
}

fn checkpoint_path(directory: &Path, serial: u64) -> PathBuf {
    directory.join(format!("checkpoint-{serial:020}.json"))
}

#[derive(Clone, Copy)]
pub(super) struct CheckpointFile {
    serial: u64,
    automatic: bool,
}

impl CheckpointFile {
    fn path(self, directory: &Path) -> PathBuf {
        if self.automatic { directory.join(format!("autosave-{:020}.json", self.serial)) }
        else { checkpoint_path(directory, self.serial) }
    }
}

pub(super) fn committed_checkpoints(directory: &Path) -> Result<Vec<CheckpointFile>, RuntimeError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_error("enumerate checkpoints", directory, err)),
    };
    let mut serials = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| io_error("read checkpoint directory entry", directory, err))?;
        let name = entry.file_name();
        let Some(name) = name.to_str().and_then(|name| name.strip_suffix(".json")) else { continue };
        let (digits, automatic) = if let Some(digits) = name.strip_prefix("checkpoint-") { (digits, false) }
            else if let Some(digits) = name.strip_prefix("autosave-") { (digits, true) }
            else { continue };
        if digits.len() == 20 && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            if let Ok(serial) = digits.parse::<u64>() {
                serials.push(CheckpointFile { serial, automatic });
            }
        }
    }
    serials.sort_unstable_by_key(|entry| (entry.serial, !entry.automatic));
    Ok(serials)
}

fn publish_checkpoint(
    mut file: File,
    candidate: &Path,
    destination: &Path,
    envelope: &Envelope,
    before_publish: impl FnOnce() -> io::Result<()>,
) -> Result<(), RuntimeError> {
    let bytes = serde_json::to_vec(envelope).map_err(|err| serialization_error("serialize checkpoint", err))?;
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
        return Err(RuntimeError::CheckpointIo { details: "检查点超出当前格式支持的大小，上一份提交仍可读取".into() });
    }
    file.write_all(&bytes)
        .map_err(|err| io_error("write checkpoint candidate", candidate, err))?;
    file.sync_all().map_err(|err| io_error("flush checkpoint candidate", candidate, err))?;
    drop(file);
    before_publish().map_err(|err| io_error("publish checkpoint", destination, err))?;
    // Publish the flushed file under a new name without replacing any prior entry.
    // A hard link is a one-step, no-replace operation on the local target filesystem.
    fs::hard_link(candidate, destination).map_err(|err| io_error("publish checkpoint", destination, err))?;
    Ok(())
}

fn read_checkpoint(path: &Path, world_id: &str, serial: u64) -> Result<StoredWorld, ReadError> {
    let file = File::open(path).map_err(|err| ReadError::Io(io_error("open checkpoint", path, err)))?;
    let mut bytes = Vec::new();
    file.take(MAX_CHECKPOINT_BYTES + 1).read_to_end(&mut bytes)
        .map_err(|err| ReadError::Io(io_error("read checkpoint", path, err)))?;
    let corrupt = |details: String| ReadError::Corrupt(format!("checkpoint {serial}: {details}"));
    if bytes.len() as u64 > MAX_CHECKPOINT_BYTES {
        return Err(corrupt("file is larger than the supported fixture format".to_owned()));
    }
    let header: Header = serde_json::from_slice(&bytes).map_err(|err| corrupt(err.to_string()))?;
    if header.magic != MAGIC {
        return Err(corrupt("unexpected file marker".to_owned()));
    }
    if !matches!(header.format_version, LEGACY_FORMAT_VERSION | FORMAT_VERSION) {
        return Err(ReadError::Unsupported(format!("checkpoint {serial} format version {}", header.format_version)));
    }
    let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|err| corrupt(err.to_string()))?;
    if envelope.checkpoint_serial != serial || envelope.checksum != checkpoint_checksum(serial, envelope.payload.as_bytes()) {
        return Err(corrupt("checkpoint checksum or serial mismatch".to_owned()));
    }
    let identity: PayloadHeader = serde_json::from_str(&envelope.payload)
        .map_err(|err| corrupt(err.to_string()))?;
    if !WorldState::supports_definition(identity.state.model_version, &identity.state.fixture_definition) {
        return Err(ReadError::Unsupported(format!(
            "checkpoint {serial} model {} and definition {}",
            identity.state.model_version, identity.state.fixture_definition,
        )));
    }
    if identity.solar_config.is_some_and(|solar| solar.model_version != SOLAR_MODEL_VERSION) {
        return Err(ReadError::Unsupported(format!("checkpoint {serial} has an unsupported solar model version")));
    }
    let world: StoredWorld = serde_json::from_str(&envelope.payload).map_err(|err| corrupt(err.to_string()))?;
    if world.world_id != world_id {
        return Err(corrupt("world ID does not match the containing directory".to_owned()));
    }
    if world.solar_config.as_ref().is_some_and(|solar| !solar.location.validate()) {
        return Err(corrupt("invalid solar location".to_owned()));
    }
    if !world.observation_history.valid(&world.state) {
        return Err(corrupt("invalid observation history".to_owned()));
    }
    match world.state.validate() {
        Ok(()) => Ok(world),
        Err(CoreError::UnsupportedModel) => Err(ReadError::Unsupported(format!("checkpoint {serial} uses an unsupported world definition or model"))),
        Err(error) => Err(corrupt(error.to_string())),
    }
}

fn checkpoint_checksum(serial: u64, payload: &[u8]) -> String {
    // FNV-1a detects accidental corruption. This is not an authenticity check.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in serial.to_le_bytes().into_iter().chain(payload.iter().copied()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn serialization_error(operation: &str, error: serde_json::Error) -> RuntimeError {
    RuntimeError::CheckpointIo { details: format!("{operation}: {error}") }
}

fn io_error(operation: &str, path: &Path, error: io::Error) -> RuntimeError {
    RuntimeError::CheckpointIo { details: format!("{operation} at {}: {error}", path.display()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldRuntime;

    fn with_temp_store(test: impl FnOnce(SaveStore)) {
        struct TempDirectory {
            path: PathBuf,
            parent: PathBuf,
        }
        impl Drop for TempDirectory {
            fn drop(&mut self) {
                if let Ok(path) = self.path.canonicalize() {
                    if path.parent() == Some(&self.parent) {
                        let _ = fs::remove_dir_all(path);
                    }
                }
            }
        }

        let parent = std::env::temp_dir().canonicalize().unwrap();
        let path = parent.join(format!("micro-ecosystem-m0-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        let temporary = TempDirectory { path, parent };
        test(SaveStore::new(&temporary.path));
    }

    #[test]
    fn fresh_runtime_restores_exact_state_and_random_continuation() {
        with_temp_store(|store| {
            let mut original = WorldRuntime::new();
            let start = original.create_fixture("forest", 719_521).unwrap();
            original.set_paused("forest", true).unwrap();
            original.edit_rock("forest", &start.run_generation, "0", 0.700_503, 0.67).unwrap();
            original.set_paused("forest", false).unwrap();
            original.advance_ticks("forest", 23).unwrap();
            original.set_daylight_target("forest", 0.04).unwrap();
            original.advance_ticks("forest", 31).unwrap();
            original.set_paused("forest", true).unwrap();
            original.save(&store, "forest").unwrap();

            let complete = &original.world("forest").unwrap().saved;
            let mut restored = WorldRuntime::new();
            let loaded = restored.load(&store, "forest").unwrap().snapshot;
            assert_eq!(loaded.simulation_tick, "54");
            assert_eq!(loaded.edit_revision, "1");
            assert_ne!(loaded.run_generation, start.run_generation);
            assert!(loaded.paused);
            assert_eq!(&restored.world("forest").unwrap().saved, complete);
            assert_eq!(restored.advance_ticks("forest", 33).unwrap(), loaded);
            assert!(matches!(restored.edit_rock("forest", &start.run_generation, "1", 0.5, 0.5), Err(RuntimeError::StaleGeneration { .. })));

            for runtime in [&mut original, &mut restored] {
                runtime.set_paused("forest", false).unwrap();
                runtime.advance_ticks("forest", 166).unwrap();
            }
            assert_eq!(restored.world("forest").unwrap().saved, original.world("forest").unwrap().saved);
        });
    }

    #[test]
    fn save_failure_leaves_last_commit_and_ignores_uncommitted_candidate() {
        with_temp_store(|store| {
            let mut runtime = WorldRuntime::new();
            runtime.create_fixture("forest", 4).unwrap();
            runtime.advance_ticks("forest", 10).unwrap();
            runtime.save(&store, "forest").unwrap();

            runtime.advance_ticks("forest", 30).unwrap();
            let state = &runtime.world("forest").unwrap().saved;
            let result = store.save_before_publish(state, false, || Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected publish failure")));
            assert!(matches!(result, Err(RuntimeError::CheckpointIo { .. })));

            let directory = store.world_directory("forest");
            fs::write(directory.join("pending-interrupted.tmp"), b"\x01\x02").unwrap();
            let committed = runtime.load(&store, "forest").unwrap();
            assert_eq!(committed.snapshot.simulation_tick, "10");
            assert_eq!(committed.checkpoint_serial, "1");
            assert!(!committed.recovered_previous);
        });
    }

    #[test]
    fn corrupt_newest_recovers_previous_and_can_publish_a_new_serial() {
        with_temp_store(|store| {
            let mut runtime = WorldRuntime::new();
            runtime.create_fixture("forest", 4).unwrap();
            runtime.advance_ticks("forest", 10).unwrap();
            runtime.save(&store, "forest").unwrap();
            runtime.advance_ticks("forest", 30).unwrap();
            runtime.save(&store, "forest").unwrap();

            let damaged = checkpoint_path(&store.world_directory("forest"), 2);
            let mut envelope: Envelope = serde_json::from_slice(&fs::read(&damaged).unwrap()).unwrap();
            envelope.payload = envelope.payload.replace("\"tick\":40", "\"tick\":41");
            let damaged_content = serde_json::to_vec(&envelope).unwrap();
            fs::write(&damaged, &damaged_content).unwrap();

            let restored = runtime.load(&store, "forest").unwrap();
            assert_eq!(restored.snapshot.simulation_tick, "10");
            assert!(restored.recovered_previous);
            assert_eq!(restored.checkpoint_serial, "1");
            assert_eq!(runtime.save(&store, "forest").unwrap().checkpoint_serial, "3");
            assert_eq!(fs::read(&damaged).unwrap(), damaged_content);
        });
    }

    #[test]
    fn future_checkpoint_fails_without_loading_an_older_world() {
        with_temp_store(|store| {
            let mut runtime = WorldRuntime::new();
            runtime.create_fixture("forest", 4).unwrap();
            runtime.save(&store, "forest").unwrap();
            let current = runtime.advance_ticks("forest", 30).unwrap();
            let future = checkpoint_path(&store.world_directory("forest"), 2);
            fs::write(&future, br#"{"magic":"micro-ecosystem-fixture","formatVersion":3}"#).unwrap();
            assert!(matches!(runtime.load(&store, "forest"), Err(RuntimeError::CheckpointUnsupported { .. })));
            assert_eq!(runtime.snapshot("forest").unwrap(), current);
        });
    }

    #[test]
    fn unfamiliar_model_payload_is_not_treated_as_corruption() {
        with_temp_store(|store| {
            let mut runtime = WorldRuntime::new();
            runtime.create_fixture("forest", 4).unwrap();
            runtime.save(&store, "forest").unwrap();
            let current = runtime.advance_ticks("forest", 30).unwrap();
            let payload = r#"{"state":{"modelVersion":2,"fixtureDefinition":"forest-fixture/2","futureFields":[]}}"#.to_owned();
            let future = Envelope {
                magic: MAGIC.to_owned(),
                format_version: FORMAT_VERSION,
                checkpoint_serial: 2,
                checksum: checkpoint_checksum(2, payload.as_bytes()),
                payload,
            };
            let path = checkpoint_path(&store.world_directory("forest"), 2);
            fs::write(path, serde_json::to_vec(&future).unwrap()).unwrap();
            assert!(matches!(runtime.load(&store, "forest"), Err(RuntimeError::CheckpointUnsupported { .. })));
            assert_eq!(runtime.snapshot("forest").unwrap(), current);
        });
    }

    #[test]
    fn validates_identity_and_distinguishes_absent_and_unusable() {
        with_temp_store(|store| {
            let mut runtime = WorldRuntime::new();
            assert_eq!(runtime.load(&store, "../forest"), Err(RuntimeError::InvalidWorldId));
            assert!(matches!(runtime.load(&store, "forest"), Err(RuntimeError::CheckpointMissing { .. })));
            let original = runtime.create_fixture("forest", 4).unwrap();
            runtime.save(&store, "forest").unwrap();
            let damaged = checkpoint_path(&store.world_directory("forest"), 1);
            fs::write(&damaged, b"{").unwrap();
            assert!(matches!(runtime.load(&store, "forest"), Err(RuntimeError::CheckpointUnusable { .. })));
            assert_eq!(runtime.snapshot("forest").unwrap(), original);
        });
    }
}
