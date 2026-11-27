use crate::checkpoint::committed_checkpoints;
use crate::{validate_world_id, RuntimeError, SaveStore, Snapshot, SolarConfig, WorldRuntime};
use eco_core::{WorldState, FOREST_R5_COMPOSED_MODEL_VERSION, POND_MODEL_VERSION, POND_R5_COMPOSED_MODEL_VERSION, POND_R5_MOSS_MODEL_VERSION, POND_R5_STONES_MODEL_VERSION,
    POND_R5_WOOD_MODEL_VERSION};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use uuid::Uuid;

const VERSION: u32 = 1;
const MAX_ENTRY_BYTES: u64 = 2048;
const MAX_PRESET_BYTES: u64 = 256 * 1024;
const ENTRY_FILENAME: &str = "library-entry.json";
const DEFAULT_SELECTION_FILENAME: &str = "r5-default-selection.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorldTheme {
    Forest,
    Pond,
}

impl WorldTheme {
    pub fn for_model_version(model_version: u32) -> Self {
        if matches!(model_version, POND_MODEL_VERSION | POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION
            | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION) {
            Self::Pond
        } else {
            Self::Forest
        }
    }

    fn id(self) -> &'static str {
        match self { Self::Forest => "forest", Self::Pond => "pond" }
    }

    pub fn current_model_version(self) -> u32 {
        match self { Self::Forest => FOREST_R5_COMPOSED_MODEL_VERSION, Self::Pond => POND_R5_COMPOSED_MODEL_VERSION }
    }

    pub fn default_preset_id(self) -> &'static str {
        match self { Self::Forest => "forest-r5-composed", Self::Pond => "pond-r5-composed" }
    }

    fn title(self, model_version: u32) -> &'static str {
        match (self, model_version == self.current_model_version()) {
            (Self::Forest, true) => "林下 · R5 分层造景",
            (Self::Pond, true) => "浅水池塘 · R5 水下造景",
            (Self::Forest, false) => "旧版林下切片",
            (Self::Pond, false) => "旧版浅水池塘",
        }
    }
}

impl Snapshot {
    pub fn default_world_name(&self) -> &'static str {
        WorldTheme::for_model_version(self.model_version).title(self.model_version)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorldEntryState {
    Ready,
    Recovered,
    Unreadable,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldEntry {
    pub world_id: String,
    pub name: String,
    pub archived: bool,
    pub model_version: Option<u32>,
    pub definition: Option<String>,
    pub theme: Option<WorldTheme>,
    pub paused: Option<bool>,
    pub simulation_tick: Option<String>,
    pub state: WorldEntryState,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PresetSource {
    BuiltIn,
    Blank,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEntry {
    pub preset_id: String,
    pub name: String,
    pub theme: WorldTheme,
    pub source: PresetSource,
    pub model_version: u32,
}

const BUILT_INS: [(&str, &str, WorldTheme, PresetSource, u32); 6] = [
    ("forest-r5-composed", "林下 · R5 分层造景", WorldTheme::Forest, PresetSource::BuiltIn, FOREST_R5_COMPOSED_MODEL_VERSION),
    ("blank-forest-r5-composed", "空白林下 · R5 分层造景", WorldTheme::Forest, PresetSource::Blank, FOREST_R5_COMPOSED_MODEL_VERSION),
    ("pond-r5-composed", "浅水池塘 · R5 水下造景", WorldTheme::Pond, PresetSource::BuiltIn, POND_R5_COMPOSED_MODEL_VERSION),
    ("pond-r5-dual", "浅水池塘 · R5 双水域", WorldTheme::Pond, PresetSource::BuiltIn, POND_R5_COMPOSED_MODEL_VERSION),
    ("blank-pond-r5-composed", "空白浅塘 · R5 水下造景", WorldTheme::Pond, PresetSource::Blank, POND_R5_COMPOSED_MODEL_VERSION),
    ("blank-pond-r5-dual", "空白浅塘 · R5 双水域", WorldTheme::Pond, PresetSource::Blank, POND_R5_COMPOSED_MODEL_VERSION),
];

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EntryMetadata {
    format_version: u32,
    name: Option<String>,
    archived: bool,
}

impl Default for EntryMetadata {
    fn default() -> Self { Self { format_version: VERSION, name: None, archived: false } }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CustomPreset {
    format_version: u32,
    preset_id: String,
    name: String,
    seed: u64,
    state: WorldState,
    solar_config: Option<SolarConfig>,
}

impl CustomPreset {
    fn entry(&self) -> PresetEntry {
        PresetEntry {
            preset_id: self.preset_id.clone(), name: self.name.clone(),
            theme: WorldTheme::for_model_version(self.state.model_version()),
            source: PresetSource::Custom, model_version: self.state.model_version(),
        }
    }
}

impl SaveStore {
    pub fn validate_library_name(name: &str) -> Result<String, RuntimeError> {
        library_name(name)
    }

    pub fn has_r5_default_selection(&self) -> Result<bool, RuntimeError> {
        let path = self.root.join(DEFAULT_SELECTION_FILENAME);
        Ok(read_record::<bool>(&path, 32)?.unwrap_or(false))
    }

    /// Once initialized, later manual choices retain the usual restart behavior.
    pub fn mark_r5_default_selection(&self) -> Result<(), RuntimeError> {
        if self.has_r5_default_selection()? { return Ok(()); }
        write_record(&self.root.join(DEFAULT_SELECTION_FILENAME), &true, 32)
    }

    /// Existing version-1 checkpoints are discovered without migration writes.
    pub fn discover_worlds(&self) -> Result<Vec<WorldEntry>, RuntimeError> {
        let mut worlds = Vec::new();
        if let Some(entries) = read_directory(&self.root)? {
            for entry in entries {
                let entry = entry.map_err(|error| library_io("读取世界目录", &self.root, error))?;
                if !entry.file_type().map_err(|error| library_io("检查世界目录", &entry.path(), error))?.is_dir() { continue; }
                let filename = entry.file_name();
                let Some(world_id) = filename.to_str().and_then(|name| name.strip_prefix("world-")) else { continue; };
                if validate_world_id(world_id).is_err() { continue; }
                let checkpoint = self.load(world_id);
                if matches!(checkpoint, Err(RuntimeError::CheckpointMissing { .. })) { continue; }
                let mut row = WorldEntry {
                    world_id: world_id.to_owned(), name: world_id.to_owned(), archived: false,
                    model_version: None, definition: None, theme: None, paused: None, simulation_tick: None,
                    state: WorldEntryState::Ready, message: None,
                };
                match checkpoint {
                    Ok(saved) => {
                        let version = saved.world.state.model_version();
                        let theme = WorldTheme::for_model_version(version);
                        row.theme = Some(theme);
                        row.name = theme.title(version).to_owned();
                        row.model_version = Some(version);
                        row.definition = Some(saved.world.state.definition().to_owned());
                        row.paused = Some(saved.world.is_paused());
                        row.simulation_tick = Some(saved.world.state.tick().to_string());
                        if saved.recovered_previous {
                            row.state = WorldEntryState::Recovered;
                            row.message = Some("最新检查点损坏，将使用上一份有效检查点。".to_owned());
                        }
                    }
                    Err(RuntimeError::CheckpointUnsupported { .. }) => {
                        row.state = WorldEntryState::Unsupported;
                        row.message = Some("当前版本无法读取此世界的检查点。文件已保留。".to_owned());
                    }
                    Err(error) => {
                        row.state = WorldEntryState::Unreadable;
                        row.message = Some(format!("无法读取此世界。{error}"));
                    }
                }
                match self.entry_metadata(world_id) {
                    Ok(metadata) => {
                        row.archived = metadata.archived;
                        if let Some(name) = metadata.name { row.name = name; }
                    }
                    Err(_) => {
                        let prior = row.message.take().unwrap_or_default();
                        row.message = Some(format!("{prior}世界库记录无法读取，原记录已保留。"));
                    }
                }
                worlds.push(row);
            }
        }
        worlds.sort_by(|a, b| (a.archived, &a.name, &a.world_id).cmp(&(b.archived, &b.name, &b.world_id)));
        Ok(worlds)
    }

    pub fn set_world_name(&self, world_id: &str, name: &str) -> Result<(), RuntimeError> {
        let name = library_name(name)?;
        self.update_entry_metadata(world_id, |entry| entry.name = Some(name))
    }

    /// Archive affects the library and scheduling only. All checkpoints remain on disk.
    pub fn set_world_archived(&self, world_id: &str, archived: bool) -> Result<(), RuntimeError> {
        self.update_entry_metadata(world_id, |entry| entry.archived = archived)
    }

    fn update_entry_metadata(&self, world_id: &str, change: impl FnOnce(&mut EntryMetadata)) -> Result<(), RuntimeError> {
        validate_world_id(world_id)?;
        let directory = self.world_directory(world_id);
        let real_directory = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata.file_type().is_dir(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(library_io("检查世界目录", &directory, error)),
        };
        if !real_directory || committed_checkpoints(&directory)?.is_empty() {
            return Err(RuntimeError::CheckpointMissing { world_id: world_id.to_owned() });
        }
        let mut metadata = self.entry_metadata(world_id)?;
        change(&mut metadata);
        write_record(&directory.join(ENTRY_FILENAME), &metadata, MAX_ENTRY_BYTES)
    }

    fn entry_metadata(&self, world_id: &str) -> Result<EntryMetadata, RuntimeError> {
        let path = self.world_directory(world_id).join(ENTRY_FILENAME);
        let metadata: EntryMetadata = read_record(&path, MAX_ENTRY_BYTES)?.unwrap_or_default();
        if metadata.format_version != VERSION || metadata.name.as_ref().is_some_and(|name| library_name(name).as_ref() != Ok(name)) {
            return Err(invalid_record(&path));
        }
        Ok(metadata)
    }

    /// Unreadable user presets stay on disk and do not hide built-ins or unrelated presets.
    pub fn list_presets_with_warnings(&self) -> (Vec<PresetEntry>, Vec<String>) {
        let mut presets = BUILT_INS.into_iter().map(|(id, title, theme, source, model_version)| PresetEntry {
            preset_id: id.to_owned(), name: title.to_owned(), theme, source, model_version,
        }).collect::<Vec<_>>();
        let mut warnings = Vec::new();
        let directory = self.root.join("presets");
        let entries = match read_directory(&directory) {
            Ok(Some(entries)) => entries,
            Ok(None) => return (presets, warnings),
            Err(error) => return (presets, vec![error.to_string()]),
        };
        let mut custom = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => { warnings.push(library_io("读取预设目录", &directory, error).to_string()); continue; }
            };
            let filename = entry.file_name();
            let Some(preset_id) = filename.to_str().and_then(|name| name.strip_suffix(".json")) else { continue; };
            if !preset_id.starts_with("preset-") || validate_world_id(preset_id).is_err() { continue; }
            match self.custom_preset(preset_id) {
                Ok(preset) => custom.push(preset.entry()),
                Err(error) => warnings.push(error.to_string()),
            }
        }
        custom.sort_by(|a, b| (&a.name, &a.preset_id).cmp(&(&b.name, &b.preset_id)));
        presets.extend(custom);
        (presets, warnings)
    }

    fn custom_preset(&self, preset_id: &str) -> Result<CustomPreset, RuntimeError> {
        validate_world_id(preset_id)?;
        let directory = self.root.join("presets");
        if let Ok(metadata) = fs::symlink_metadata(&directory) {
            if !metadata.file_type().is_dir() { return Err(invalid_record(&directory)); }
        }
        let path = directory.join(format!("{preset_id}.json"));
        let preset: CustomPreset = read_record(&path, MAX_PRESET_BYTES)?.ok_or_else(|| RuntimeError::PresetMissing { preset_id: preset_id.to_owned() })?;
        if preset.format_version != VERSION || preset.preset_id != preset_id
            || library_name(&preset.name).as_ref() != Ok(&preset.name) || preset.state.tick() != 0
            || preset.solar_config.as_ref().is_some_and(|solar| !solar.location.validate() || solar.model_version != super::SOLAR_MODEL_VERSION)
            || preset.state.validate().is_err()
        {
            return Err(invalid_record(&path));
        }
        Ok(preset)
    }
}

impl WorldRuntime {
    pub fn is_world_loaded(&self, world_id: &str) -> bool {
        self.worlds.contains_key(world_id) && !self.trials.contains_key(world_id)
    }

    pub fn world_is_paused(&self, world_id: &str) -> Result<bool, RuntimeError> {
        Ok(self.world(world_id)?.saved.is_paused())
    }

    pub fn create_library_world(&mut self, store: &SaveStore, preset_id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        let (state, solar_config) = match preset_id {
            "forest-r5-composed" | "forest" | "forest-r5" | "forest-r5-terrain" | "forest-r5-stonework"
            | "forest-r5-bark" | "forest-r5-litter" => (WorldState::new_r5_composed_forest(seed), default_solar()),
            "pond-r5-composed" | "pond" | "pond-r5-stones" | "pond-r5-wood" | "pond-r5-moss" =>
                (WorldState::new_r5_composed_pond(seed), default_solar()),
            "pond-r5-dual" => (WorldState::new_r5_composed_dual_pond(seed), default_solar()),
            "blank-forest-r5-composed" | "blank-forest" => (WorldState::new_blank_r5_composed_forest(seed), default_solar()),
            "blank-pond-r5-composed" | "blank-pond" => (WorldState::new_blank_r5_composed_pond(seed), default_solar()),
            "blank-pond-r5-dual" => (WorldState::new_blank_r5_composed_dual_pond(seed), default_solar()),
            id if id.starts_with("preset-") => {
                let preset = store.custom_preset(id)?;
                (preset.state, preset.solar_config)
            }
            _ => return Err(RuntimeError::PresetMissing { preset_id: preset_id.to_owned() }),
        };
        self.create_library_state(state, solar_config)
    }

    fn create_library_state(&mut self, state: WorldState, solar_config: Option<SolarConfig>) -> Result<Snapshot, RuntimeError> {
        let theme = WorldTheme::for_model_version(state.model_version());
        let world_id = format!("{}-{}", theme.id(), Uuid::new_v4().simple());
        self.create(&world_id, state, solar_config)?;
        self.set_edit_paused(&world_id, true)
    }

    #[cfg(test)]
    pub(crate) fn create_legacy_library_fixture(&mut self, id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        let state = match id {
            "forest-r5" => WorldState::new_r5_forest(seed),
            "forest-r5-terrain" => WorldState::new_r5_terrain_forest(seed),
            "forest-r5-stonework" => WorldState::new_r5_stonework_forest(seed),
            "forest-r5-bark" => WorldState::new_r5_bark_forest(seed),
            "forest-r5-litter" => WorldState::new_r5_leaf_forest(seed),
            "pond" => WorldState::new_pond(seed),
            "pond-r5-stones" => WorldState::new_r5_stone_pond(seed),
            "pond-r5-wood" => WorldState::new_r5_wood_pond(seed),
            "pond-r5-moss" => WorldState::new_r5_moss_pond(seed),
            _ => panic!("unknown legacy library fixture: {id}"),
        };
        self.create_library_state(state, default_solar())
    }

    pub fn save_custom_preset(&self, store: &SaveStore, world_id: &str, name: &str, seed: u64) -> Result<PresetEntry, RuntimeError> {
        let name = library_name(name)?;
        if !self.is_world_loaded(world_id) { return Err(RuntimeError::WorldMissing { world_id: world_id.to_owned() }); }
        let world = &self.world(world_id)?.saved;
        let state = world.state.fresh_habitat(seed)?;
        let preset_id = format!("preset-{}", Uuid::new_v4().simple());
        let preset = CustomPreset { format_version: VERSION, preset_id, name, seed, state, solar_config: world.solar_config.clone() };
        let path = store.root.join("presets").join(format!("{}.json", preset.preset_id));
        write_record(&path, &preset, MAX_PRESET_BYTES)?;
        Ok(preset.entry())
    }
}

fn default_solar() -> Option<SolarConfig> {
    Some(super::default_solar_config())
}

fn library_name(name: &str) -> Result<String, RuntimeError> {
    let name = name.trim();
    if !(1..=80).contains(&name.chars().count()) || name.chars().any(char::is_control) {
        return Err(RuntimeError::InvalidLibraryName);
    }
    Ok(name.to_owned())
}

fn read_directory(path: &Path) -> Result<Option<fs::ReadDir>, RuntimeError> {
    match fs::read_dir(path) {
        Ok(entries) => Ok(Some(entries)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(library_io("读取资料库目录", path, error)),
    }
}

fn read_record<T: DeserializeOwned>(path: &Path, maximum: u64) -> Result<Option<T>, RuntimeError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(library_io("检查资料库记录", path, error)),
    };
    if !metadata.file_type().is_file() || metadata.len() > maximum { return Err(invalid_record(path)); }
    let file = File::open(path).map_err(|error| library_io("读取资料库记录", path, error))?;
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes).map_err(|error| library_io("读取资料库记录", path, error))?;
    if bytes.len() as u64 > maximum { return Err(invalid_record(path)); }
    serde_json::from_slice(&bytes).map(Some).map_err(|_| invalid_record(path))
}

fn write_record(path: &Path, value: &impl Serialize, maximum: u64) -> Result<(), RuntimeError> {
    let bytes = serde_json::to_vec(value).map_err(|_| invalid_record(path))?;
    if bytes.len() as u64 > maximum { return Err(invalid_record(path)); }
    let parent = path.parent().expect("record has a parent directory");
    fs::create_dir_all(parent).map_err(|error| library_io("建立资料库目录", parent, error))?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| library_io("检查资料库目录", parent, error))?;
    if !metadata.file_type().is_dir() { return Err(invalid_record(parent)); }
    let candidate = parent.join(format!("library-{}.tmp", Uuid::new_v4()));
    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&candidate)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&candidate, path)
    })();
    if result.is_err() { let _ = fs::remove_file(&candidate); }
    result.map_err(|error| library_io("提交资料库记录", path, error))
}

fn invalid_record(path: &Path) -> RuntimeError {
    RuntimeError::LibraryUnreadable { details: format!("资料库记录损坏、超出大小限制或版本不兼容，已保留原文件。{}", path.display()) }
}

fn library_io(operation: &str, path: &Path, error: io::Error) -> RuntimeError {
    RuntimeError::LibraryUnreadable { details: format!("{operation}失败。{} · {error}", path.display()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PondEdit;
    use eco_core::POND_DEFAULT_BASIN_ID;

    #[test]
    fn starter_introduction_is_explicit_paused_once_and_accounted() {
        let store = SaveStore::new(std::env::temp_dir());
        let mut runtime = WorldRuntime::new();
        let forest = runtime.create_library_world(&store, "blank-forest", 1206).unwrap();
        runtime.set_edit_paused(&forest.world_id, false).unwrap();
        runtime.advance_ticks(&forest.world_id, 601).unwrap();
        let paused = runtime.set_paused(&forest.world_id, true).unwrap();
        assert!(paused.starter_community_available && paused.manual_paused);
        assert_eq!(runtime.add_starter_community_command(&forest.world_id, "manual", &forest.run_generation, "0"),
            Err(RuntimeError::EditRequiresPause));
        let before = runtime.set_edit_paused(&forest.world_id, true).unwrap();
        let updated = runtime.add_starter_community_command(&forest.world_id, "introduce", &forest.run_generation, "0").unwrap();
        assert_eq!(updated.animals.len(), 4);
        assert_eq!(updated.rock, before.rock);
        assert_eq!(updated.simulation_tick, before.simulation_tick);
        assert_eq!(updated.forest.as_ref().unwrap().water_ml, before.forest.as_ref().unwrap().water_ml);
        assert!(!updated.starter_community_available);
        let ledger = runtime.forest_ledger(&forest.world_id).unwrap().unwrap();
        assert!(ledger.matter_input_mg_cnp.iter().all(|mass| *mass > 0.0));
        assert_eq!(runtime.add_starter_community_command(&forest.world_id, "introduce", &forest.run_generation, "0").unwrap(), updated);
        assert!(matches!(runtime.add_starter_community_command(&forest.world_id, "duplicate", &forest.run_generation, "1"),
            Err(RuntimeError::InvalidFixtureInput { .. })));

        let pond = runtime.create_library_world(&store, "blank-pond", 2729).unwrap();
        assert!(matches!(runtime.add_starter_community_command(&pond.world_id, "dry", &pond.run_generation, "0"),
            Err(RuntimeError::InvalidFixtureInput { .. })));
        let filled = runtime.edit_pond_command(&pond.world_id, "water", &pond.run_generation, "0",
            PondEdit::AddWater { basin_id: POND_DEFAULT_BASIN_ID.to_owned(), liters: 18.0 }).unwrap();
        let made = runtime.add_starter_community_command(&pond.world_id, "wet", &pond.run_generation, "1").unwrap();
        let before = &filled.pond.as_ref().unwrap().basins[0];
        let after = &made.pond.as_ref().unwrap().basins[0];
        assert_eq!(after.water_liters, before.water_liters);
        assert_eq!(after.oxygen_mg_per_liter, before.oxygen_mg_per_liter);
        assert_eq!(after.shrimp.len(), 3);
        assert_eq!(after.daphnia_count, 48);
        let source = &runtime.world(&pond.world_id).unwrap().saved.state;
        source.validate().unwrap();
        let preset = source.fresh_habitat(2729).unwrap();
        assert_eq!(preset.tick(), 0);
        assert!(!preset.starter_community_available());
        assert_eq!(preset.pond_observation().unwrap().basins[0].daphnia_count, 48);
        assert_eq!(preset.pond_ledger().unwrap().basins[0].matter_input_mg_cnp, [0.0; 3]);
        let undone = runtime.edit_pond_history_command(&pond.world_id, "undo", &pond.run_generation, "2", false).unwrap();
        let mut expected = filled.pond.unwrap();
        // Undo restores the habitat while retaining the allocated shrimp ID range.
        expected.basins[0].retain_observation_row = Some(true);
        assert_eq!(undone.pond, Some(expected));
        assert!(undone.starter_community_available);
    }

    #[test]
    fn library_discovers_legacy_worlds_and_preserves_unreadable_checkpoints() {
        let root = std::env::temp_dir().join(format!("micro-ecosystem-library-{}", Uuid::new_v4()));
        let store = SaveStore::new(&root);
        let mut runtime = WorldRuntime::new();
        runtime.create_forest("legacy", 1206).unwrap();
        runtime.save(&store, "legacy").unwrap();
        runtime.save(&store, "legacy").unwrap();
        runtime.create_pond("damaged", 2729).unwrap();
        runtime.save(&store, "damaged").unwrap();
        let newer = store.world_directory("legacy").join("checkpoint-00000000000000000002.json");
        let damaged = store.world_directory("damaged").join("checkpoint-00000000000000000001.json");
        fs::write(&newer, b"partial checkpoint").unwrap();
        fs::write(&damaged, b"damaged checkpoint").unwrap();
        assert_eq!(store.set_world_name("legacy", " \t "), Err(RuntimeError::InvalidLibraryName));
        store.set_world_name("legacy", "  老林下  ").unwrap();
        store.set_world_archived("legacy", true).unwrap();

        let reopened = SaveStore::new(&root);
        let rows = reopened.discover_worlds().unwrap();
        let legacy = rows.iter().find(|row| row.world_id == "legacy").unwrap();
        assert_eq!(legacy.name, "老林下");
        assert_eq!(legacy.model_version, Some(2));
        assert_eq!(legacy.state, WorldEntryState::Recovered);
        assert!(legacy.archived);
        let unreadable = rows.iter().find(|row| row.world_id == "damaged").unwrap();
        assert_eq!(unreadable.state, WorldEntryState::Unreadable);
        assert!(unreadable.message.is_some());
        assert_eq!(fs::read(&newer).unwrap(), b"partial checkpoint");
        assert_eq!(fs::read(&damaged).unwrap(), b"damaged checkpoint");
        reopened.set_world_archived("legacy", false).unwrap();
        assert!(!reopened.discover_worlds().unwrap().iter().find(|row| row.world_id == "legacy").unwrap().archived);
        // Every path removed here belongs to this uniquely named test fixture.
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blank_presets_and_custom_habitat_have_independent_valid_initial_state() {
        let root = std::env::temp_dir().join(format!("micro-ecosystem-presets-{}", Uuid::new_v4()));
        let store = SaveStore::new(&root);
        let mut runtime = WorldRuntime::new();
        let forest = runtime.create_library_world(&store, "blank-forest", 1206).unwrap();
        assert!(forest.edit_paused && forest.animals.is_empty());
        let forest_stocks = forest.forest.unwrap();
        let life = forest_stocks.life.unwrap();
        assert_eq!(forest_stocks.litter_mg_c, 0.0);
        assert_eq!(life.springtail_mg_c, [0.0; 2]);
        assert_eq!(life.fern_mg_c, [0.0; 2]);
        assert_eq!(life.moss_mg_c, [0.0; 2]);
        runtime.save(&store, &forest.world_id).unwrap();

        let pond = runtime.create_library_world(&store, "blank-pond", 2729).unwrap();
        let basin = &pond.pond.as_ref().unwrap().basins[0];
        assert!(!basin.wet && basin.shrimp.is_empty() && basin.daphnia_count == 0);
        assert_eq!(basin.oxygen_mg_per_liter, None);
        let basin_id = POND_DEFAULT_BASIN_ID.to_owned();
        let filled = runtime.edit_pond_command(&pond.world_id, "add-water", &pond.run_generation, "0",
            PondEdit::AddWater { basin_id: basin_id.clone(), liters: 2.5 }).unwrap();
        runtime.edit_pond_command(&pond.world_id, "litter", &pond.run_generation, &filled.edit_revision,
            PondEdit::AddDetritus { basin_id, mg_c: 18.0 }).unwrap();
        runtime.set_edit_paused(&pond.world_id, false).unwrap();
        let advanced = runtime.advance_ticks(&pond.world_id, 601).unwrap();
        let custom = runtime.save_custom_preset(&store, &pond.world_id, "空白小塘", 2729).unwrap();
        assert_eq!(runtime.snapshot(&pond.world_id).unwrap(), advanced);

        let store = SaveStore::new(&root);
        let (presets, warnings) = store.list_presets_with_warnings();
        assert!(warnings.is_empty());
        assert_eq!(presets.last(), Some(&custom));
        let first = runtime.create_library_world(&store, &custom.preset_id, 1).unwrap();
        let second = runtime.create_library_world(&store, &custom.preset_id, 2).unwrap();
        assert_ne!(first.world_id, second.world_id);
        let first_state = &runtime.world(&first.world_id).unwrap().saved.state;
        assert_eq!(first_state, &runtime.world(&second.world_id).unwrap().saved.state);
        assert_eq!(first.simulation_tick, "0");
        let initial = &first.pond.unwrap().basins[0];
        let source = &advanced.pond.unwrap().basins[0];
        assert_eq!(initial.water_liters, source.water_liters);
        assert_eq!(initial.surface_area_cm2, source.surface_area_cm2);
        assert_eq!(initial.detritus_mg_c, 0.0);
        assert!(source.detritus_mg_c > initial.detritus_mg_c);
        assert!(initial.shrimp.is_empty() && initial.daphnia_count == 0);
        assert_eq!(first_state.pond_ledger().unwrap().basins[0].water_input_liters, 0.0);
        let corrupt = root.join("presets").join("preset-corrupt.json");
        fs::write(&corrupt, b"broken preset").unwrap();
        let (presets, warnings) = store.list_presets_with_warnings();
        assert_eq!(presets.len(), BUILT_INS.len() + 1);
        assert_eq!(presets.last(), Some(&custom));
        assert_eq!(warnings.len(), 1);
        assert_eq!(fs::read(&corrupt).unwrap(), b"broken preset");
        // Every path removed here belongs to this uniquely named test fixture.
        fs::remove_dir_all(root).unwrap();
    }
}
