use super::*;
use world_runtime::{PresetEntry, WorldBackupImport, WorldEntry, WorldEntryState, WorldTheme};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    active_world_id: String,
    worlds: Vec<WorldEntry>,
    presets: Vec<PresetEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    preset_warnings: Vec<String>,
}

pub(super) fn select_r5_default(
    store: &SaveStore, runtime: &mut WorldRuntime, active: &Snapshot, entries: &[WorldEntry],
) -> Result<Option<Snapshot>, RuntimeError> {
    let theme = WorldTheme::for_model_version(active.model_version);
    if active.model_version == theme.current_model_version() || store.has_r5_default_selection()? { return Ok(None); }
    let existing = entries.iter().find(|entry| !entry.archived &&
        entry.model_version == Some(theme.current_model_version()) &&
        matches!(entry.state, WorldEntryState::Ready | WorldEntryState::Recovered));
    let selected = match existing {
        Some(entry) => runtime.load(store, &entry.world_id)?.snapshot,
        None => {
            let seed = if theme == WorldTheme::Forest { FOREST_SEED } else { POND_SEED };
            let snapshot = runtime.create_library_world(store, theme.default_preset_id(), seed)?;
            let id = &snapshot.world_id;
            let saved = runtime.set_edit_paused(id, false).and_then(|running| {
                runtime.save(store, id)?;
                Ok(running)
            });
            if saved.is_err() { runtime.unload(id); }
            saved?
        }
    };
    if let Err(error) = store.select_active_world(&selected.world_id) {
        runtime.unload(&selected.world_id);
        return Err(error);
    }
    if let Err(error) = store.mark_r5_default_selection() { eprintln!("记录 R5 默认选择失败：{error}"); }
    Ok(Some(selected))
}

impl Host {
    pub fn list_worlds(&self) -> HostResult<LibrarySnapshot> {
        let (presets, preset_warnings) = self.store.list_presets_with_warnings();
        Ok(LibrarySnapshot {
            active_world_id: self.world.world.world_id.clone(),
            worlds: self.world_entries()?,
            presets,
            preset_warnings,
        })
    }

    fn world_entries(&self) -> HostResult<Vec<WorldEntry>> {
        let mut worlds = self.store.discover_worlds()?;
        let active = &self.world.world;
        if !worlds.iter().any(|entry| entry.world_id == active.world_id) {
            worlds.push(WorldEntry {
                world_id: active.world_id.clone(),
                name: self.world.world_name.clone(),
                archived: false,
                model_version: Some(active.model_version),
                definition: Some(active.definition.clone()),
                theme: Some(if active.pond.is_some() { WorldTheme::Pond } else { WorldTheme::Forest }),
                state: if self.save_blocked.is_some() { WorldEntryState::Unreadable } else { WorldEntryState::Ready },
                message: self.save_blocked.clone(),
                paused: Some(active.paused),
                simulation_tick: Some(active.simulation_tick.clone()),
            });
        }
        for entry in &mut worlds {
            if matches!(entry.state, WorldEntryState::Ready | WorldEntryState::Recovered) {
                if let Ok(snapshot) = self.runtime.snapshot(&entry.world_id) {
                    entry.paused = Some(snapshot.paused);
                    entry.simulation_tick = Some(snapshot.simulation_tick);
                }
            }
        }
        Ok(worlds)
    }

    pub fn switch_world(&mut self, id: &str) -> HostResult<ViewSnapshot> {
        self.require_library_change()?;
        if id == self.world.world.world_id { return Ok(self.world.clone()); }
        let entry = self.library_entry(id)?;
        if entry.archived { return Err(HostError::new("world_archived", "请先从归档恢复该世界")); }
        self.save_before_switch()?;
        if !self.runtime.is_world_loaded(id) {
            self.runtime.load(&self.store, id)?;
            self.autosaves.remember(&self.runtime, id);
        }
        let snapshot = self.runtime.snapshot(id)?;
        self.store.select_active_world(id)?;
        Self::remember_manual_selection(&self.store, &snapshot);
        self.bind_world(snapshot, entry.name, Some("已切换世界".into()));
        self.trace("world-switched", json!({}));
        Ok(self.world.clone())
    }

    pub fn create_world(&mut self, preset_id: &str, name: &str) -> HostResult<ViewSnapshot> {
        self.require_library_change()?;
        let name = SaveStore::validate_library_name(name)?;
        self.save_before_switch()?;
        let seed = if preset_id.starts_with("pond") || preset_id.starts_with("blank-pond") { POND_SEED } else { FOREST_SEED };
        let snapshot = self.runtime.create_library_world(&self.store, preset_id, seed)?;
        self.finish_new_world(snapshot, name, "已从初始方案创建并保存新世界", true)
    }

    pub fn copy_world(&mut self, id: &str, name: &str) -> HostResult<ViewSnapshot> {
        self.require_library_change()?;
        let name = SaveStore::validate_library_name(name)?;
        let entry = self.library_entry(id)?;
        if id == self.world.world.world_id && self.save_blocked.is_some() {
            return Err(HostError::new("save_blocked", "当前只显示临时恢复画面，无法复制原世界"));
        }
        if entry.archived { return Err(HostError::new("world_archived", "请先从归档恢复该世界")); }
        self.save_before_switch()?;
        if !self.runtime.is_world_loaded(id) { self.runtime.load(&self.store, id)?; }
        let snapshot = self.runtime.copy_world(&self.store, id)?;
        self.finish_new_world(snapshot, name, "已复制当下状态，副本保持暂停", false)
    }

    pub fn export_world_backup(&self, id: &str) -> HostResult<String> {
        self.require_library_change()?;
        if id == self.world.world.world_id && self.save_blocked.is_some() {
            return Err(HostError::new("save_blocked", "当前恢复画面不可作为原世界备份"));
        }
        let entry = self.library_entry(id)?;
        Ok(self.runtime.export_world_backup(&self.store, id, &entry.name)?)
    }

    pub fn import_world_backup(&self, contents: &str) -> HostResult<WorldBackupImport> {
        self.require_library_change()?;
        Ok(self.store.import_world_backup(contents)?)
    }

    pub fn rename_world(&mut self, id: &str, name: &str) -> HostResult<ViewSnapshot> {
        self.require_library_change()?;
        let name = SaveStore::validate_library_name(name)?;
        let _entry = self.library_entry(id)?;
        if id == self.world.world.world_id && self.save_blocked.is_none() && self.store.discover_worlds()?.iter().all(|entry| entry.world_id != id) {
            self.save()?;
        }
        self.store.set_world_name(id, &name)?;
        self.world_names.insert(id.into(), name.clone());
        if id == self.world.world.world_id {
            self.world.world_name = name;
        }
        self.bump_sequence();
        Ok(self.world.clone())
    }

    pub fn archive_world(&mut self, id: &str, archived: bool) -> HostResult<ViewSnapshot> {
        self.require_library_change()?;
        if id == self.world.world.world_id {
            return Err(HostError::new("world_active", "请先切换到另一世界，再归档此世界"));
        }
        let entry = self.library_entry(id)?;
        if entry.archived == archived { return Ok(self.world.clone()); }
        if archived && self.runtime.is_world_loaded(id) {
            self.runtime.save(&self.store, id)?;
        }
        self.store.set_world_archived(id, archived)?;
        if archived {
            self.autosaves.forget(id);
            self.background.retain(|(world_id, _)| world_id != id);
            self.runtime.unload(id);
        } else if self.runtime.load(&self.store, id).is_ok() {
            self.autosaves.remember(&self.runtime, id);
            self.background.push_back((id.into(), 0));
        }
        self.trace("world-archive-changed", json!({ "affectedWorldId": id, "archived": archived }));
        self.bump_sequence();
        Ok(self.world.clone())
    }

    pub fn save_world_preset(&mut self, name: &str) -> HostResult<PresetEntry> {
        self.require_library_change()?;
        if self.save_blocked.is_some() {
            return Err(HostError::new("save_blocked", "当前恢复画面不可用于生成预设"));
        }
        let id = &self.world.world.world_id;
        let seed = if self.world.world.pond.is_some() { POND_SEED } else { FOREST_SEED };
        Ok(self.runtime.save_custom_preset(&self.store, id, &SaveStore::validate_library_name(name)?, seed)?)
    }

    pub fn adopt_trial(&mut self) -> HostResult<ViewSnapshot> {
        if self.save_blocked.is_some() {
            return Err(HostError::new("save_blocked", "当前恢复画面不可覆盖原存档"));
        }
        let now = power::active_time_100ns();
        self.sample(now, power::stamp());
        let id = self.trial_id.as_deref().ok_or(RuntimeError::TrialMissing)?;
        let snapshot = self.runtime.apply_trial(&self.store, id)?;
        self.trial_id = None;
        self.subscriptions.clear();
        self.active_backlog_100ns = 0;
        self.autosaves.remember(&self.runtime, &snapshot.world_id);
        self.world.last_save_message = Some("已采用试运行结果并保存检查点".into());
        self.publish(snapshot);
        self.refresh_solar(now);
        self.trace("trial-adopted", json!({}));
        Ok(self.world.clone())
    }

    pub fn save_trial_as_world(&mut self, name: &str) -> HostResult<ViewSnapshot> {
        let name = SaveStore::validate_library_name(name)?;
        let id = self.trial_id.clone().ok_or(RuntimeError::TrialMissing)?;
        self.save_before_switch()?;
        let snapshot = self.runtime.save_trial_as_world(&self.store, &id)?;
        self.finish_new_world(snapshot, name, "已将试运行结果另存为独立世界", false)?;
        self.runtime.discard_trial(&id)?;
        self.trial_id = None;
        self.refresh_solar(power::active_time_100ns());
        self.trace("trial-saved-as-world", json!({}));
        Ok(self.world.clone())
    }

    fn library_entry(&self, id: &str) -> HostResult<WorldEntry> {
        self.world_entries()?.into_iter().find(|entry| entry.world_id == id)
            .ok_or_else(|| RuntimeError::WorldMissing { world_id: id.to_owned() }.into())
    }

    pub(super) fn save_before_switch(&mut self) -> HostResult<()> {
        self.sample(power::active_time_100ns(), power::stamp());
        if self.world.suspended { return Err(RuntimeError::Suspended.into()); }
        if self.save_blocked.is_none() { self.save()?; }
        Ok(())
    }

    fn require_library_change(&self) -> HostResult<()> {
        self.require_formal_world()?;
        if self.world.suspended { return Err(RuntimeError::Suspended.into()); }
        Ok(())
    }

    fn finish_new_world(&mut self, snapshot: Snapshot, name: String, message: &str, needs_save: bool) -> HostResult<ViewSnapshot> {
        let id = &snapshot.world_id;
        let save = (|| -> Result<(), RuntimeError> {
            if needs_save { self.runtime.save(&self.store, id)?; }
            self.store.set_world_name(id, &name)?;
            self.store.select_active_world(id)?;
            Ok(())
        })();
        if let Err(error) = save {
            self.runtime.unload(id);
            return Err(error.into());
        }
        Self::remember_manual_selection(&self.store, &snapshot);
        self.autosaves.remember(&self.runtime, id);
        self.bind_world(snapshot, name, Some(message.into()));
        self.trace("world-created", json!({}));
        Ok(self.world.clone())
    }

    fn remember_manual_selection(store: &SaveStore, snapshot: &Snapshot) {
        let model_version = snapshot.model_version;
        if model_version == WorldTheme::for_model_version(model_version).current_model_version() { return; }
        if let Err(error) = store.mark_r5_default_selection() { eprintln!("记录手动选择旧世界失败：{error}"); }
    }

    pub(super) fn bind_world(&mut self, snapshot: Snapshot, name: String, message: Option<String>) {
        let previous = std::mem::take(&mut self.active_backlog_100ns);
        if let Some(index) = self.background.iter().position(|(id, _)| id == &snapshot.world_id) {
            self.active_backlog_100ns = self.background.remove(index).expect("world index is current").1;
        }
        let old_id = &self.world.world.world_id;
        if self.save_blocked.is_none() {
            self.background.push_back((old_id.clone(), previous));
        } else {
            self.autosaves.forget(old_id);
            self.runtime.unload(old_id);
        }
        self.subscriptions.clear();
        self.save_blocked = None;
        self.world_names.insert(snapshot.world_id.clone(), name.clone());
        self.world.world_name = name;
        self.world.last_save_message = self.autosaves.failure(&snapshot.world_id).or(message);
        self.publish(snapshot);
        self.refresh_solar(power::active_time_100ns());
    }
}
