use crate::power;
mod autosave;
mod library;
use autosave::Autosaves;
pub use library::LibrarySnapshot;
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use world_runtime::solar::{self, SolarLocation, SolarObservation};
use world_runtime::{ForestRockCommand, PondEdit, RuntimeError, SaveStore, Snapshot, TrialRate, WorldRuntime, STEP_MILLIS};

const DEFAULT_WORLD_ID: &str = "forest-understory";
pub const VIEW_IDS: [&str; 3] = ["editor", "observe", "panel"];
const FOREST_SEED: u64 = 1206;
const POND_SEED: u64 = 2729;
const SOLAR_REFRESH_100NS: u64 = 150_000_000;
const STEP_100NS: u64 = STEP_MILLIS * 10_000;
const MAX_STEPS_PER_WAKE: u64 = 20;
const MAX_BACKGROUND_STEPS_PER_WAKE: u32 = 40;
const MAX_BACKGROUND_STEPS_PER_WORLD: u32 = 10;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(25);

pub type HostResult<T> = Result<T, HostError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostError {
    pub code: &'static str,
    pub message: String,
}

impl HostError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl From<RuntimeError> for HostError {
    fn from(error: RuntimeError) -> Self {
        let code = match &error {
            RuntimeError::CheckpointMissing { .. } => "checkpoint_missing",
            RuntimeError::CheckpointUnusable { .. } => "checkpoint_unusable",
            RuntimeError::CommandReuse => "command_id_reused",
            RuntimeError::StaleGeneration { .. } => "stale_generation",
            RuntimeError::StaleRevision { .. } => "stale_revision",
            RuntimeError::EditRequiresPause => "edit_requires_pause",
            RuntimeError::InvalidLocation => "invalid_location",
            RuntimeError::Suspended => "suspended",
            RuntimeError::InvalidFixtureInput { .. } => "invalid_fixture_input",
            RuntimeError::PondShape { issue } => issue.code(),
            RuntimeError::CheckpointUnsupported { .. } => "checkpoint_unsupported",
            RuntimeError::TrialConflict => "trial_conflict",
            RuntimeError::InvalidLibraryName => "invalid_library_name",
            RuntimeError::PresetMissing { .. } => "preset_missing",
            RuntimeError::LibraryUnreadable { .. } => "library_unreadable",
            _ => "world_operation_failed",
        };
        Self::new(code, error.to_string())
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewSnapshot {
    #[serde(flatten)]
    world: Snapshot,
    sequence: String,
    world_name: String,
    last_save_message: Option<String>,
    suspended: bool,
    active_backlog_millis: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    solar: Option<SolarObservation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    subscription_token: String,
    snapshot: ViewSnapshot,
}

struct ViewBinding {
    token: String,
    world_id: Option<String>,
}

#[derive(Clone)]
pub struct SharedHost(Arc<Mutex<Host>>);

impl SharedHost {
    pub fn new(host: Host) -> Self {
        Self(Arc::new(Mutex::new(host)))
    }

    pub fn lock(&self) -> HostResult<MutexGuard<'_, Host>> {
        self.0.lock().map_err(|_| HostError::new("runtime_unavailable", "运行层状态暂不可用"))
    }

    pub fn start_clock(&self) -> std::io::Result<()> {
        let shared = self.clone();
        thread::Builder::new().name("ecology-active-clock".into()).spawn(move || {
            let registration = power::Registration::new();
            if let Ok(mut host) = shared.lock() {
                host.trace("power-registration", json!({ "registered": registration.is_ok(), "win32Error": registration.as_ref().err() }));
            }
            loop {
                let deadline_missed = {
                    let Ok(mut host) = shared.lock() else { break };
                    if host.exiting { break; }
                    host.sample(power::active_time_100ns(), power::stamp());
                    host.has_pending_ticks()
                };
                // Drain a limited group of steps per lock, giving control and IPC
                // a chance to run even if a long awake stall has queued work.
                if deadline_missed { thread::yield_now(); }
                thread::sleep(SAMPLE_INTERVAL);
            }
            drop(registration);
        })?;
        Ok(())
    }
}

pub struct Host {
    runtime: WorldRuntime,
    store: SaveStore,
    world: ViewSnapshot,
    serial: u64,
    next_subscription: u64,
    subscriptions: HashMap<String, ViewBinding>,
    world_names: HashMap<String, String>,
    visible: HashMap<String, bool>,
    previous_active: Option<u64>,
    active_backlog_100ns: u64,
    background: VecDeque<(String, u64)>,
    autosaves: Autosaves,
    previous_power: u64,
    next_solar_sample: u64,
    next_background_solar_sample: u64,
    trial_id: Option<String>,
    next_trace_sample: u64,
    save_blocked: Option<String>,
    trace_file: Option<File>,
    exiting: bool,
}

impl Host {
    pub fn open(path: PathBuf, trace_path: Option<PathBuf>) -> HostResult<Self> {
        let store = SaveStore::new(path);
        let mut runtime = WorldRuntime::new();
        let previous_selection = store.active_world();
        let world_id = previous_selection.as_ref().ok().and_then(Option::as_deref).unwrap_or(DEFAULT_WORLD_ID);
        let missing_is_new = matches!(&previous_selection, Ok(None));
        let loaded = match &previous_selection {
            Ok(_) => runtime.load(&store, world_id),
            Err(error) => Err(error.clone()),
        };
        let (mut snapshot, mut boot_mode, mut save_message, save_blocked) = match loaded {
            Ok(receipt) => {
                let recovered = receipt.recovered_previous;
                let message = if recovered { "已从上一有效检查点恢复" }
                    else if receipt.automatic { "已从自动检查点恢复" } else { "已恢复已保存的世界" };
                (receipt.snapshot, if recovered { "fallback" } else { "restored" }, Some(message.to_owned()), None)
            }
            Err(RuntimeError::CheckpointMissing { .. }) if missing_is_new => (
                runtime.create_composed_forest(world_id, FOREST_SEED)?, "new-preset", None, None,
            ),
            Err(read_error) => {
                // Permit inspection of an in-memory preset while preserving the
                // unreadable existing files until a valid version can be loaded.
                let message = format!("现有世界无法读取。暂时显示林下预设，已禁用覆盖保存。{read_error}");
                (runtime.create_composed_forest(world_id, FOREST_SEED)?, "unusable-save", Some(message.clone()), Some(message))
            }
        };
        let entries = store.discover_worlds();
        if let Ok(entries) = &entries {
            if matches!(boot_mode, "restored" | "fallback") {
                match library::select_r5_default(&store, &mut runtime, &snapshot, entries) {
                    Ok(Some(current)) => {
                        snapshot = current;
                        boot_mode = "r5-default";
                        save_message = Some("已打开 R5 分层世界。旧世界仍可在世界库打开。".to_owned());
                    }
                    Ok(None) => {}
                    Err(error) => { save_message = Some(format!("继续显示已保存的世界。无法切换 R5 默认世界：{error}")); }
                }
            }
        }
        let world_id = snapshot.world_id.as_str();
        let mut background = VecDeque::new();
        let mut autosaves = Autosaves::new();
        if matches!(boot_mode, "restored" | "fallback" | "r5-default") { autosaves.remember(&runtime, world_id); }
        let mut world_names = HashMap::new();
        let mut world_name = snapshot.default_world_name().to_owned();
        if let Ok(entries) = entries {
            for entry in entries {
                world_names.insert(entry.world_id.clone(), entry.name.clone());
                if entry.world_id == world_id {
                    if save_blocked.is_none() { world_name = entry.name; }
                } else if !entry.archived && (runtime.is_world_loaded(&entry.world_id) ||
                    runtime.load(&store, &entry.world_id).is_ok()) {
                    autosaves.remember(&runtime, &entry.world_id);
                    background.push_back((entry.world_id, 0));
                }
            }
        }
        let trace_file = trace_path.and_then(|path| {
            OpenOptions::new().create(true).append(true).open(path).map_err(|error| {
                eprintln!("ecology trace open failed: {error}");
            }).ok()
        });
        let mut host = Self {
            runtime,
            store,
            world: ViewSnapshot {
                world: snapshot,
                sequence: "1".into(),
                world_name,
                last_save_message: save_message,
                suspended: false,
                active_backlog_millis: 0,
                solar: None,
            },
            serial: 1,
            next_subscription: 0,
            subscriptions: HashMap::new(),
            world_names,
            visible: VIEW_IDS.into_iter().map(|label| (label.to_owned(), false)).collect(),
            previous_active: None,
            active_backlog_100ns: 0,
            background,
            autosaves,
            previous_power: 0,
            next_solar_sample: 0,
            next_background_solar_sample: 0,
            trial_id: None,
            next_trace_sample: 0,
            save_blocked,
            trace_file,
            exiting: false,
        };
        host.refresh_solar(power::active_time_100ns());
        host.trace("runtime-started", json!({ "bootMode": boot_mode, "solar": host.world.solar }));
        Ok(host)
    }

    pub fn record_visibility(&mut self, label: &str, visible: bool) {
        let changed = self.visible.insert(label.into(), visible) != Some(visible);
        if !visible { self.subscriptions.retain(|key, _| key.split(':').next() != Some(label)); }
        if changed { self.trace("window-visibility", json!({ "viewId": label, "visible": visible })); }
    }

    #[cfg(test)]
    fn subscribe(&mut self, label: &str) -> HostResult<Subscription> {
        self.subscribe_to(label, None)
    }

    pub fn subscribe_to(&mut self, label: &str, world_id: Option<&str>) -> HostResult<Subscription> {
        let physical = label.split(':').next().unwrap_or(label);
        if !self.visible.get(physical).copied().unwrap_or(false) {
            return Err(HostError::new("view_hidden", "这个窗口当前处于隐藏状态"));
        }
        if world_id.is_some() && physical != "panel" {
            return Err(HostError::new("invalid_view", "此窗口随编辑器切换世界"));
        }
        let snapshot = self.snapshot_for_view(world_id)?;
        self.next_subscription += 1;
        let token = format!("{}:{}:{}", snapshot.world.run_generation, label, self.next_subscription);
        self.subscriptions.insert(label.to_owned(), ViewBinding { token: token.clone(), world_id: world_id.map(str::to_owned) });
        self.trace("view-subscribed", json!({ "viewId": physical, "subscriptionToken": token, "componentSlot": label.split_once(':').map(|(_, slot)| slot) }));
        Ok(Subscription { subscription_token: token, snapshot })
    }

    pub fn poll(&self, label: &str, token: &str, after_sequence: Option<&str>) -> HostResult<Option<ViewSnapshot>> {
        let binding = self.subscriptions.get(label).filter(|binding| binding.token == token)
            .ok_or_else(|| HostError::new("stale_subscription", "此视图已关闭或需要重新连接到当前世界"))?;
        let known = after_sequence.map(str::parse::<u64>).transpose()
            .map_err(|_| HostError::new("invalid_sequence", "快照序号无效"))?;
        if known.is_some_and(|sequence| sequence > self.serial) {
            return Err(HostError::new("stale_subscription", "客户端握有属于另一次连接的快照"));
        }
        if known == Some(self.serial) { return Ok(None); }
        self.snapshot_for_view(binding.world_id.as_deref()).map(Some)
    }

    pub fn unsubscribe(&mut self, label: &str, token: &str) {
        if self.subscriptions.get(label).is_some_and(|binding| binding.token == token) {
            self.subscriptions.remove(label);
            self.trace("view-unsubscribed", json!({ "viewId": label.split(':').next().unwrap_or(label) }));
        }
    }

    fn snapshot_for_view(&self, world_id: Option<&str>) -> HostResult<ViewSnapshot> {
        let Some(id) = world_id.filter(|id| *id != self.world.world.world_id) else { return Ok(self.world.clone()); };
        let unavailable = || HostError::new("bound_world_unavailable", "组件绑定的世界不可用。可从托盘的组件设置更换来源。");
        if !self.background.iter().any(|(background, _)| background == id) { return Err(unavailable()); }
        let world = self.runtime.snapshot(id).map_err(|_| unavailable())?;
        let world_name = self.world_names.get(id).cloned().unwrap_or_else(|| world.default_world_name().into());
        Ok(ViewSnapshot { world, world_name, last_save_message: self.autosaves.failure(id), active_backlog_millis: 0, solar: None, ..self.world.clone() })
    }

    pub fn pause(&mut self, paused: bool) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        if self.world.world.manual_paused != paused {
            // Account for the awake interval before the pause boundary rather
            // than carrying it over into the resumed session.
            self.sample(power::active_time_100ns(), power::stamp());
            let snapshot = self.runtime.set_paused(&self.world.world.world_id, paused)?;
            self.previous_active = Some(power::active_time_100ns());
            self.publish(snapshot);
            self.trace("pause-changed", json!({ "paused": paused }));
        }
        Ok(self.world.clone())
    }

    pub fn pause_for_editing(&mut self, paused: bool) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        if self.world.world.edit_paused != paused {
            self.sample(power::active_time_100ns(), power::stamp());
            let snapshot = self.runtime.set_edit_paused(&self.world.world.world_id, paused)?;
            self.previous_active = Some(power::active_time_100ns());
            self.publish(snapshot);
            self.trace("edit-pause-changed", json!({ "paused": paused }));
        }
        Ok(self.world.clone())
    }

    pub fn edit(&mut self, command_id: &str, generation: &str, revision: &str, x: f64, y: f64) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        if generation != self.world.world.run_generation {
            return Err(HostError::new("stale_generation", "此操作属于已经结束的世界运行代次"));
        }
        let id = &self.world.world.world_id;
        self.runtime.edit_rock_command(id, command_id, generation, revision, x, y)?;
        let current = self.runtime.snapshot(id)?;
        if current.edit_revision != self.world.world.edit_revision {
            self.publish(current);
            self.trace("rock-edited", json!({ "commandId": command_id, "x": x, "y": y }));
        }
        // A successful retry may acknowledge an older revision. Always send
        // the current snapshot instead of moving any subscriber backwards.
        Ok(self.world.clone())
    }

    fn pond_edit_target(&self, requested_world: Option<&str>) -> HostResult<String> {
        let current = self.trial_id.as_deref().unwrap_or(&self.world.world.world_id);
        if let Some(requested) = requested_world {
            if requested != current {
                return Err(HostError::new("stale_generation", "此操作属于已经结束的世界或试运行"));
            }
        } else {
            self.require_formal_world()?;
        }
        Ok(current.to_owned())
    }

    pub fn edit_pond(&mut self, world_id: Option<&str>, command_id: &str, generation: &str, revision: &str, edit: PondEdit) -> HostResult<ViewSnapshot> {
        let id = self.pond_edit_target(world_id)?;
        self.runtime.edit_pond_command(&id, command_id, generation, revision, edit)?;
        let current = self.runtime.snapshot(&id)?;
        if self.trial_id.is_some() {
            return Ok(self.trial_view(current));
        }
        if current.edit_revision != self.world.world.edit_revision {
            self.publish(current);
            self.trace("pond-edited", json!({ "commandId": command_id }));
        }
        Ok(self.world.clone())
    }

    pub fn edit_forest_rocks(
        &mut self, world_id: &str, command_id: &str, generation: &str, revision: &str, action: ForestRockCommand,
    ) -> HostResult<ViewSnapshot> {
        let expected_world = self.trial_id.as_deref().unwrap_or(&self.world.world.world_id);
        if world_id != expected_world {
            return Err(HostError::new("stale_generation", "此操作属于已经结束的世界或试运行"));
        }
        self.runtime.edit_forest_rock_command(world_id, command_id, generation, revision, action)?;
        let current = self.runtime.snapshot(world_id)?;
        if self.trial_id.is_some() {
            return Ok(self.trial_view(current));
        }
        if current.edit_revision != self.world.world.edit_revision {
            self.publish(current);
            self.trace("forest-rocks-edited", json!({ "commandId": command_id }));
        }
        Ok(self.world.clone())
    }

    pub fn edit_pond_history(&mut self, world_id: Option<&str>, command_id: &str, generation: &str, revision: &str, redo: bool) -> HostResult<ViewSnapshot> {
        let id = self.pond_edit_target(world_id)?;
        self.runtime.edit_pond_history_command(&id, command_id, generation, revision, redo)?;
        let current = self.runtime.snapshot(&id)?;
        if self.trial_id.is_some() {
            return Ok(self.trial_view(current));
        }
        if current.edit_revision != self.world.world.edit_revision {
            self.publish(current);
            self.refresh_solar(power::active_time_100ns());
            self.trace("pond-history", json!({ "redo": redo, "commandId": command_id }));
        }
        Ok(self.world.clone())
    }

    pub fn add_starter_life(&mut self, command_id: &str, generation: &str, revision: &str) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        let id = &self.world.world.world_id;
        self.runtime.add_starter_community_command(id, command_id, generation, revision)?;
        let current = self.runtime.snapshot(id)?;
        if current.edit_revision != self.world.world.edit_revision {
            self.publish(current);
            self.trace("starter-community-added", json!({ "commandId": command_id }));
        }
        Ok(self.world.clone())
    }

    pub fn save(&mut self) -> HostResult<ViewSnapshot> {
        if let Some(message) = &self.save_blocked {
            return Err(HostError::new("save_blocked", message.clone()));
        }
        match self.runtime.save(&self.store, &self.world.world.world_id) {
            Ok(receipt) => {
                self.autosaves.remember(&self.runtime, &receipt.world_id);
                self.world.last_save_message = Some(format!("已保存第 {} 步", receipt.simulation_tick));
                self.bump_sequence();
                self.trace("checkpoint-saved", json!({ "checkpointSerial": receipt.checkpoint_serial }));
                Ok(self.world.clone())
            }
            Err(error) => {
                let message = format!("保存失败。{error}");
                self.world.last_save_message = Some(message.clone());
                self.bump_sequence();
                self.trace("checkpoint-save-failed", json!({ "message": message }));
                Err(HostError::new("save_failed", message))
            }
        }
    }

    pub fn restore(&mut self) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        let receipt = self.runtime.load(&self.store, &self.world.world.world_id)?;
        self.subscriptions.clear();
        self.previous_active = Some(power::active_time_100ns());
        self.active_backlog_100ns = 0;
        self.save_blocked = None;
        self.autosaves.remember(&self.runtime, &receipt.snapshot.world_id);
        self.world.last_save_message = Some(if receipt.recovered_previous { "已从上一有效检查点恢复" }
            else if receipt.automatic { "已从自动检查点恢复" } else { "已加载检查点" }.into());
        self.publish(receipt.snapshot);
        self.refresh_solar(power::active_time_100ns());
        self.trace("checkpoint-restored", json!({ "recoveredPrevious": receipt.recovered_previous }));
        Ok(self.world.clone())
    }

    pub fn prepare_exit(&mut self) -> HostResult<()> {
        self.sample(power::active_time_100ns(), power::stamp());
        if self.save_blocked.is_none() { self.save()?; }
        for (id, _) in &self.background {
            if let Err(error) = self.runtime.save(&self.store, id) {
                let message = format!("后台世界 {id} 保存失败。{error}");
                self.world.last_save_message = Some(message.clone());
                self.bump_sequence();
                return Err(HostError::new("save_failed", message));
            }
        }
        self.exiting = true;
        self.trace("exit-request-committed", json!({ "skippedUnavailableActiveWorld": self.save_blocked.is_some() }));
        Ok(())
    }

    pub fn is_exiting(&self) -> bool {
        self.exiting
    }

    fn require_formal_world(&self) -> HostResult<()> {
        if self.trial_id.is_some() {
            return Err(HostError::new("trial_active", "请先结束当前试运行"));
        }
        Ok(())
    }

    pub fn set_location(&mut self, location: SolarLocation) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        let snapshot = self.runtime.set_location(&self.world.world.world_id, location)?;
        self.publish(snapshot);
        self.refresh_solar(power::active_time_100ns());
        self.trace("solar-location-changed", json!({}));
        Ok(self.world.clone())
    }

    pub fn create_from_preset(&mut self, pond: bool) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        self.save_before_switch()?;
        let (preset_id, preset_name, seed) = if pond { ("pond-r5-composed", "浅塘", POND_SEED) }
            else { ("forest-r5-composed", "林下", FOREST_SEED) };
        let snapshot = self.runtime.create_library_world(&self.store, preset_id, seed)?;
        let id = &snapshot.world_id;
        if let Err(error) = self.runtime.save(&self.store, id).and_then(|_| self.store.select_active_world(id)) {
            self.runtime.unload(id);
            return Err(error.into());
        }
        let name = snapshot.default_world_name().to_owned();
        self.autosaves.remember(&self.runtime, &snapshot.world_id);
        self.bind_world(snapshot, name, Some(format!("已从{preset_name}预设建立并保存新世界")));
        self.trace("preset-created", json!({ "preset": preset_name }));
        Ok(self.world.clone())
    }

    pub fn start_trial(&mut self) -> HostResult<ViewSnapshot> {
        self.require_formal_world()?;
        let trial = self.runtime.start_trial(&self.world.world.world_id)?;
        self.trial_id = Some(trial.world_id.clone());
        self.trace("trial-started", json!({ "trialId": trial.world_id }));
        Ok(self.trial_view(trial))
    }

    pub fn observation_history(&self, world_id: &str, expected_generation: &str) -> HostResult<world_runtime::ObservationHistory> {
        if world_id != self.world.world.world_id && self.trial_id.as_deref() != Some(world_id) {
            return Err(HostError::new("observation_world_changed", "当前观察世界已切换"));
        }
        Ok(self.runtime.observation_history(world_id, expected_generation)?)
    }

    pub fn advance_trial(&mut self, ticks: u32) -> HostResult<ViewSnapshot> {
        let id = self.trial_id.as_deref().ok_or(RuntimeError::TrialMissing)?;
        let trial = self.runtime.advance_trial(id, ticks)?;
        self.trace("trial-advanced", json!({ "trialId": trial.world_id, "ticks": ticks, "trialTick": trial.simulation_tick }));
        Ok(self.trial_view(trial))
    }

    pub fn advance_trial_scaled(&mut self, elapsed_ticks: u32, rate: TrialRate) -> HostResult<ViewSnapshot> {
        let id = self.trial_id.as_deref().ok_or(RuntimeError::TrialMissing)?;
        let trial = self.runtime.advance_trial_scaled(id, elapsed_ticks, rate)?;
        self.trace("trial-advanced-scaled", json!({ "trialId": trial.world_id, "elapsedTicks": elapsed_ticks, "rate": rate, "trialTick": trial.simulation_tick }));
        Ok(self.trial_view(trial))
    }

    pub fn discard_trial(&mut self) -> HostResult<ViewSnapshot> {
        let id = self.trial_id.as_deref().ok_or(RuntimeError::TrialMissing)?;
        self.runtime.discard_trial(id)?;
        self.trial_id = None;
        self.trace("trial-discarded", json!({}));
        Ok(self.world.clone())
    }

    fn trial_view(&self, world: Snapshot) -> ViewSnapshot {
        ViewSnapshot { world, world_name: format!("{} · 试运行", self.world.world_name), last_save_message: None, ..self.world.clone() }
    }

    fn refresh_solar(&mut self, active_now: u64) {
        self.next_solar_sample = active_now.saturating_add(SOLAR_REFRESH_100NS);
        let utc = SystemTime::now().duration_since(UNIX_EPOCH).ok().and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok());
        self.world.solar = utc.and_then(|value| self.world.world.location.as_ref().and_then(|site| solar::observe(site, value)));
        if let Some(solar) = &self.world.solar {
            if self.trial_id.is_some() { return; }
            if let Err(error) = self.runtime.set_daylight_target(&self.world.world.world_id, solar.target_phase) {
                self.world.last_save_message = Some(format!("无法更新昼夜目标。{error}"));
            }
        }
    }

    fn has_pending_ticks(&self) -> bool {
        !self.world.suspended && (
            !self.world.world.paused && self.active_backlog_100ns >= STEP_100NS ||
            self.background.iter().any(|(id, ticks)| *ticks >= STEP_100NS && self.runtime.world_is_paused(id) == Ok(false))
        )
    }

    fn sample_background(&mut self, elapsed: u64, update_solar: bool) -> bool {
        let mut budget = MAX_BACKGROUND_STEPS_PER_WAKE;
        let mut changed_bound_world = false;
        let utc = update_solar.then(|| SystemTime::now().duration_since(UNIX_EPOCH).ok())
            .flatten().and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok());
        for (id, backlog) in &mut self.background {
            if self.runtime.world_is_paused(id) == Ok(false) {
                *backlog = backlog.saturating_add(elapsed);
            }
            if let Some(utc) = utc {
                if let Ok(snapshot) = self.runtime.snapshot(id) {
                    if let Some(target) = snapshot.location.as_ref().and_then(|site| solar::observe(site, utc)) {
                        let _ = self.runtime.set_daylight_target(id, target.target_phase);
                    }
                }
            }
        }
        for _ in 0..self.background.len() {
            if budget == 0 { break; }
            let (id, mut backlog) = self.background.pop_front().expect("background queue length stays constant");
            let due = (backlog / STEP_100NS).min(u64::from(budget.min(MAX_BACKGROUND_STEPS_PER_WORLD))) as u32;
            if due > 0 && self.runtime.world_is_paused(&id) == Ok(false) {
                match self.runtime.advance_ticks(&id, due) {
                    Ok(_) => {
                        backlog -= u64::from(due) * STEP_100NS;
                        budget -= due;
                    }
                    Err(error) => {
                        let _ = self.runtime.set_paused(&id, true);
                        self.trace("background-step-failed", json!({ "worldId": id, "message": error.to_string() }));
                    }
                }
                changed_bound_world |= self.subscriptions.values().any(|binding| binding.world_id.as_deref() == Some(id.as_str()));
            }
            self.background.push_back((id, backlog));
        }
        changed_bound_world
    }

    fn sample(&mut self, active_now: u64, power_now: u64) {
        let mut status_changed = false;
        if power_now != self.previous_power {
            self.previous_power = power_now;
            self.previous_active = None;
            self.world.suspended = power_now & 1 != 0;
            self.runtime.set_suspended(self.world.suspended);
            if self.world.suspended {
                if let Ok(snapshot) = self.runtime.snapshot(&self.world.world.world_id) {
                    self.world.world = snapshot;
                }
            } else {
                self.next_solar_sample = 0;
                self.next_background_solar_sample = 0;
            }
            status_changed = true;
            if self.trace_file.is_some() {
                let background = self.background.iter().filter_map(|(id, _)| self.runtime.snapshot(id).ok())
                    .map(|world| json!({ "worldId": world.world_id, "tick": world.simulation_tick, "paused": world.paused })).collect::<Vec<_>>();
                self.trace("power-transition", json!({ "stamp": power_now, "suspended": self.world.suspended,
                    "activeClock100ns": active_now, "background": background }));
            }
        }

        let update_solar = !self.world.suspended && active_now >= self.next_solar_sample;
        if update_solar && self.trial_id.is_none() {
            self.refresh_solar(active_now);
            status_changed = true;
        }

        let active_elapsed = self.previous_active.replace(active_now).map_or(0, |previous| active_now.saturating_sub(previous));
        if !self.world.suspended {
            let update_background_solar = active_now >= self.next_background_solar_sample;
            if update_background_solar { self.next_background_solar_sample = active_now.saturating_add(SOLAR_REFRESH_100NS); }
            status_changed |= self.sample_background(active_elapsed, update_background_solar);
        }
        if !self.world.suspended && !self.world.world.paused {
            self.active_backlog_100ns = self.active_backlog_100ns.saturating_add(active_elapsed);
            let due = (self.active_backlog_100ns / STEP_100NS).min(MAX_STEPS_PER_WAKE) as u32;
            if due > 0 {
                let world_id = &self.world.world.world_id;
                match self.runtime.advance_ticks(world_id, due) {
                    Ok(snapshot) => {
                        self.active_backlog_100ns -= u64::from(due) * STEP_100NS;
                        self.publish(snapshot);
                        status_changed = false;
                    }
                    Err(error) => {
                        // Stop attempts if a deterministic model error
                        // prevents stepping, retaining the last consistent state.
                        if let Ok(snapshot) = self.runtime.set_paused(world_id, true) {
                            self.world.world = snapshot;
                        }
                        self.world.last_save_message = Some(format!("生态步进已暂停。{error}"));
                        self.trace("runtime-step-failed", json!({ "message": error.to_string() }));
                        status_changed = true;
                    }
                }
            }
        }
        if status_changed { self.bump_sequence(); }
        if !self.world.suspended { self.sample_autosaves(active_elapsed); }
        if self.trace_file.is_some() && active_now >= self.next_trace_sample {
            self.next_trace_sample = active_now.saturating_add(10_000_000);
            self.trace("active-clock-sample", json!({ "activeClock100ns": active_now, "backlogMillis": self.active_backlog_100ns / 10_000 }));
        }
    }

    fn publish(&mut self, snapshot: Snapshot) {
        self.world.world = snapshot;
        self.bump_sequence();
    }

    #[cfg(debug_assertions)]
    pub fn probe_snapshot(&self) -> Snapshot {
        self.world.world.clone()
    }

    #[cfg(debug_assertions)]
    pub fn probe_subscriber_count(&self) -> usize {
        self.subscriptions.len()
    }

    fn bump_sequence(&mut self) {
        self.serial += 1;
        self.world.sequence = self.serial.to_string();
        self.world.active_backlog_millis = self.active_backlog_100ns / 10_000;
    }

    pub fn trace(&mut self, event: &str, details: serde_json::Value) {
        let Some(file) = self.trace_file.as_mut() else { return };
        let wall_millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        let mut visible = self.visible.iter().filter(|(_, shown)| **shown)
            .map(|(label, _)| label.as_str()).collect::<Vec<_>>();
        visible.sort_unstable();
        let record = json!({
            "event": event,
            "processId": std::process::id(),
            "wallMillis": wall_millis,
            "worldId": self.world.world.world_id,
            "runGeneration": self.world.world.run_generation,
            "tick": self.world.world.simulation_tick,
            "paused": self.world.world.paused,
            "suspended": self.world.suspended,
            "subscriberCount": self.subscriptions.len(),
            "visibleViews": visible,
            "details": details,
        });
        if writeln!(file, "{record}").is_err() { self.trace_file = None; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh(name: &str) -> Host {
        let directory = std::env::temp_dir().join(format!("micro-ecosystem-{}-{name}", std::process::id()));
        let mut host = Host::open(directory, None).expect("fixture opens");
        host.pause(false).expect("fixture runs");
        host.record_visibility("editor", true);
        host.previous_active = None;
        host
    }

    #[test]
    fn fresh_boot_and_quick_theme_creation_use_the_composed_presets() {
        let directory = std::env::temp_dir().join(format!("micro-ecosystem-{}-v2-defaults", std::process::id()));
        let mut host = Host::open(directory.clone(), None).unwrap();
        let first = host.probe_snapshot();
        assert_eq!(first.world_id, DEFAULT_WORLD_ID);
        assert_eq!(first.definition, "forest-r5-composed/1");
        assert!(!first.paused && !first.edit_paused);

        let pond = host.create_from_preset(true).unwrap().world;
        assert_eq!(pond.definition, "pond-shallow-r5-composed/1");
        assert!(pond.edit_paused);
        let forest = host.create_from_preset(false).unwrap().world;
        assert_eq!(forest.definition, "forest-r5-composed/1");
        assert!(forest.edit_paused && forest.world_id != first.world_id);

        let library = host.list_worlds().unwrap();
        let presets = serde_json::to_value(library).unwrap();
        let listed = presets["presets"].as_array().unwrap().iter()
            .map(|preset| preset["presetId"].as_str().unwrap()).collect::<Vec<_>>();
        assert_eq!(listed, ["forest-r5-composed", "blank-forest-r5-composed",
            "pond-r5-composed", "pond-r5-dual", "blank-pond-r5-composed", "blank-pond-r5-dual"]);
        let pond_alias = host.create_world("pond", "浅塘预设").unwrap().world;
        assert_eq!(pond_alias.definition, pond.definition);
        let forest_alias = host.create_world("forest", "林下预设").unwrap().world;
        assert_eq!(forest_alias.definition, forest.definition);
        drop(host);
        let mut reopened = Host::open(directory.clone(), None).unwrap();
        assert_eq!(reopened.world.world.world_id, forest_alias.world_id);
        assert_eq!(reopened.world.world.definition, forest_alias.definition);
        assert_eq!(reopened.switch_world(&pond.world_id).unwrap().world.definition, pond.definition);
        assert_eq!(reopened.switch_world(&forest.world_id).unwrap().world.definition, forest.definition);
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn boot_selects_r5_once_preserves_legacy_checkpoints_and_blocks_unreadable_selection() {
        let directory = std::env::temp_dir().join(format!("micro-ecosystem-{}-v2-legacy-default", std::process::id()));
        let store = SaveStore::new(&directory);
        let mut runtime = WorldRuntime::new();
        let legacy = runtime.create_seasonal_forest(DEFAULT_WORLD_ID, FOREST_SEED).unwrap();
        runtime.save(&store, DEFAULT_WORLD_ID).unwrap();
        let legacy_file = directory.join(format!("world-{DEFAULT_WORLD_ID}")).join("checkpoint-00000000000000000001.json");
        let original = std::fs::read(&legacy_file).unwrap();
        let reopened = Host::open(directory.clone(), None).unwrap();
        let current_id = reopened.world.world.world_id.clone();
        assert_ne!(current_id, legacy.world_id);
        assert_eq!(reopened.world.world.definition, "forest-r5-composed/1");
        assert_eq!(store.active_world().unwrap().as_deref(), Some(current_id.as_str()));
        assert_eq!(std::fs::read(&legacy_file).unwrap(), original);
        drop(reopened);

        let mut reopened = Host::open(directory.clone(), None).unwrap();
        assert_eq!(reopened.world.world.world_id, current_id);
        assert_eq!(reopened.switch_world(&legacy.world_id).unwrap().world.definition, legacy.definition);
        drop(reopened);
        let reopened = Host::open(directory.clone(), None).unwrap();
        assert_eq!(reopened.world.world.world_id, legacy.world_id);
        assert_eq!(reopened.world.world.definition, legacy.definition);
        drop(reopened);

        let pointer = directory.join("active-world.json");
        let unreadable = b"broken world selection";
        std::fs::write(&pointer, unreadable).unwrap();
        let mut blocked = Host::open(directory.clone(), None).unwrap();
        assert_eq!(blocked.world.world.definition, "forest-r5-composed/1");
        assert_eq!(blocked.save().err().unwrap().code, "save_blocked");
        assert_eq!(std::fs::read(&pointer).unwrap(), unreadable);
        let accessible = WorldRuntime::new().load(&store, DEFAULT_WORLD_ID).unwrap();
        assert_eq!(accessible.snapshot.definition, legacy.definition);
        assert_eq!(std::fs::read(&legacy_file).unwrap(), original);
        drop(blocked);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_pond_boot_reuses_an_existing_current_pond() {
        let directory = std::env::temp_dir().join(format!("micro-ecosystem-{}-reuse-r5-pond", std::process::id()));
        let store = SaveStore::new(&directory);
        let mut runtime = WorldRuntime::new();
        let legacy = runtime.create_pond("old-pond", POND_SEED).unwrap();
        runtime.save(&store, &legacy.world_id).unwrap();
        let current = runtime.create_library_world(&store, "pond-r5-composed", POND_SEED).unwrap();
        runtime.save(&store, &current.world_id).unwrap();
        store.select_active_world(&legacy.world_id).unwrap();

        let host = Host::open(directory.clone(), None).unwrap();
        assert_eq!(host.world.world.world_id, current.world_id);
        assert_eq!(host.world.world.definition, current.definition);
        assert_eq!(store.discover_worlds().unwrap().len(), 2);
        assert_eq!(store.active_world().unwrap().as_deref(), Some(current.world_id.as_str()));
        drop(host);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn trial_leaves_formal_daylight_target_frozen_until_discard() {
        let mut host = fresh("isolated-trial-daylight");
        host.pause_for_editing(true).unwrap();
        let before = host.probe_snapshot();
        let trail = host.start_trial().unwrap();
        assert_ne!(trail.world.world_id, before.world_id);
        host.next_solar_sample = 100;
        host.sample(200, 0);
        assert_eq!(host.next_solar_sample, 100);
        host.advance_trial(600).unwrap();
        assert_eq!(host.probe_snapshot(), before);
        host.discard_trial().unwrap();
        host.sample(201, 0);
        assert!(host.next_solar_sample > 201);
        assert_eq!(host.probe_snapshot(), before);
    }

    #[test]
    fn pond_edit_authority_follows_the_explicit_r5_trial_world() {
        let mut host = fresh("pond-stone-trial");
        let formal = host.create_world("blank-pond-r5-composed", "池底试运行").unwrap().world;
        let trial = host.start_trial().unwrap().world;
        let add = PondEdit::AddStone {
            basin_id: "pond-main".into(), x: 0.37, preset: world_runtime::PondStonePreset::Main,
        };
        let wrong_world = host.edit_pond(Some(&formal.world_id), "wrong-world", &formal.run_generation, &formal.edit_revision, add.clone());
        assert_eq!(wrong_world.err().unwrap().code, "stale_generation");
        let missing_world = host.edit_pond(None, "missing-world", &trial.run_generation, &trial.edit_revision, add.clone());
        assert_eq!(missing_world.err().unwrap().code, "trial_active");
        let changed = host.edit_pond(Some(&trial.world_id), "add", &trial.run_generation, &trial.edit_revision, add.clone()).unwrap();
        let restored = host.edit_pond_history(Some(&trial.world_id), "undo", &trial.run_generation, &changed.world.edit_revision, false).unwrap();
        assert_eq!(restored.world.pond.unwrap().basins[0].stones, formal.pond.as_ref().unwrap().basins[0].stones);
        let retry = host.edit_pond(Some(&trial.world_id), "add", &trial.run_generation, &trial.edit_revision, add).unwrap();
        assert_eq!(retry.world.edit_revision, restored.world.edit_revision);
        assert_eq!(host.probe_snapshot(), formal);
        host.discard_trial().unwrap();
        assert_eq!(host.probe_snapshot(), formal);
    }

    #[test]
    fn no_view_one_view_and_two_views_share_fixed_stepping() {
        let mut states = [fresh("no-view"), fresh("one-view"), fresh("two-views")];
        states[0].record_visibility("editor", false);
        let _editor = states[1].subscribe("editor").unwrap();
        let _editor = states[2].subscribe("editor").unwrap();
        states[2].record_visibility("observe", true);
        let _observe = states[2].subscribe("observe").unwrap();
        for host in &mut states {
            host.sample(100, 0);
            host.sample(100 + 8 * STEP_100NS, 0);
        }
        for host in &states {
            assert_eq!(host.world.world.simulation_tick, "8");
            assert_eq!(host.world.world.simulated_millis, 8 * STEP_MILLIS);
            assert_eq!(
                serde_json::to_value(&host.world.world.animals).unwrap(),
                serde_json::to_value(&states[0].world.world.animals).unwrap(),
            );
            assert_eq!(host.world.world.moisture, states[0].world.world.moisture);
        }
    }

    #[test]
    fn old_binding_cannot_receive_or_remove_current_binding() {
        let mut host = fresh("reconnect");
        let original = host.subscribe("editor").unwrap();
        host.record_visibility("editor", false);
        assert_eq!(host.subscribe("editor").err().unwrap().code, "view_hidden");
        host.record_visibility("editor", true);
        let rebound = host.subscribe("editor").unwrap();
        let error = host.poll("editor", &original.subscription_token, None).err().unwrap();
        assert_eq!(error.code, "stale_subscription");
        host.unsubscribe("editor", &original.subscription_token);
        assert!(host.poll("editor", &rebound.subscription_token, None).unwrap().is_some());
    }

    #[test]
    fn slow_observer_receives_only_the_latest_complete_state() {
        let mut host = fresh("slow-observer");
        let baseline = host.subscribe("editor").unwrap();
        host.sample(100, 0);
        for tick in 1..=5 {
            host.sample(100 + tick * STEP_100NS, 0);
        }
        let latest = host.poll("editor", &baseline.subscription_token, Some(&baseline.snapshot.sequence))
            .unwrap().unwrap();
        assert_eq!(latest.world.simulation_tick, "5");
        assert_eq!(latest.world.run_generation, baseline.snapshot.world.run_generation);
        assert!(host.poll("editor", &baseline.subscription_token, Some(&latest.sequence)).unwrap().is_none());
    }

    #[test]
    fn awake_stall_drains_retained_work_in_bounded_batches() {
        let mut host = fresh("awake-stall");
        let due = MAX_STEPS_PER_WAKE + 7;
        host.sample(100, 0);
        host.sample(100 + due * STEP_100NS, 0);
        assert_eq!(host.world.world.simulated_millis, MAX_STEPS_PER_WAKE * STEP_MILLIS);
        assert_eq!(host.active_backlog_100ns, 7 * STEP_100NS);
        host.sample(100 + due * STEP_100NS, 0);
        assert_eq!(host.world.world.simulated_millis, due * STEP_MILLIS);
        assert_eq!(host.active_backlog_100ns, 0);
    }

    #[test]
    fn power_edges_do_not_add_sleep_or_remove_remainder() {
        let mut host = fresh("simulated-power-edges");
        host.sample(100, 0);
        host.sample(100 + 2 * STEP_100NS + STEP_100NS / 2, 0);
        host.sample(100 + 10 * STEP_100NS, 3);
        host.sample(100 + 12 * STEP_100NS, 4);
        assert_eq!(host.world.world.simulation_tick, "2");
        host.sample(100 + 12 * STEP_100NS + STEP_100NS / 2, 4);
        assert_eq!(host.world.world.simulation_tick, "3");
    }

    #[test]
    fn bound_panel_observes_background_ticks_without_advancing_a_paused_foreground() {
        let mut host = fresh("bound-world-background");
        let forest = host.probe_snapshot();
        let pond = host.create_world("pond", "雨后浅塘").unwrap().world;
        assert!(pond.edit_paused);
        let start = host.runtime.snapshot(&forest.world_id).unwrap();
        host.record_visibility("panel", true);
        let binding = host.subscribe_to("panel", Some(&forest.world_id)).unwrap();
        host.previous_active = None;
        host.sample(1, 0);
        host.sample(1 + 4 * STEP_100NS, 0);
        let shown = host.poll("panel", &binding.subscription_token, Some(&binding.snapshot.sequence)).unwrap().unwrap();
        assert_eq!(shown.world.world_id, forest.world_id);
        assert_eq!(shown.world.run_generation, forest.run_generation);
        assert_eq!(shown.world.simulated_millis, start.simulated_millis + 4 * STEP_MILLIS);
        assert_eq!(host.probe_snapshot(), pond);

        host.sample(1 + 100 * STEP_100NS, 3);
        host.sample(1 + 200 * STEP_100NS, 4);
        assert_eq!(host.runtime.snapshot(&forest.world_id).unwrap().simulation_tick, shown.world.simulation_tick);
        assert_eq!(host.probe_snapshot().simulation_tick, pond.simulation_tick);
        host.previous_active = None;
        let switched = host.switch_world(&forest.world_id).unwrap();
        assert_eq!(switched.world.run_generation, forest.run_generation);
        assert_eq!(switched.world.simulation_tick, shown.world.simulation_tick);
    }

    #[test]
    fn panel_card_slots_observe_independent_worlds_without_duplicate_steps() {
        let mut host = fresh("same-world-cards");
        let forest = host.probe_snapshot();
        let pond = host.create_world("pond", "观察组件背景").unwrap().world;
        let start = host.runtime.snapshot(&forest.world_id).unwrap();
        host.record_visibility("panel", true);
        let first = host.subscribe_to("panel:one", Some(&forest.world_id)).unwrap();
        let second = host.subscribe_to("panel:two", Some(&forest.world_id)).unwrap();
        let following = host.subscribe_to("panel:follow", None).unwrap();
        host.previous_active = None;
        host.sample(100, 0);
        host.sample(100 + 6 * STEP_100NS, 0);

        let left = host.poll("panel:one", &first.subscription_token, None).unwrap().unwrap();
        let right = host.poll("panel:two", &second.subscription_token, None).unwrap().unwrap();
        assert_eq!(left.world, right.world);
        assert_eq!(left.world.simulated_millis, start.simulated_millis + 6 * STEP_MILLIS);
        assert_eq!(host.poll("panel:follow", &following.subscription_token, None).unwrap().unwrap().world, pond);
        assert_eq!(host.poll("panel:two", &first.subscription_token, None).err().unwrap().code, "stale_subscription");

        host.record_visibility("panel", false);
        assert_eq!(host.probe_subscriber_count(), 0);
        assert_eq!(host.poll("panel:one", &first.subscription_token, None).err().unwrap().code, "stale_subscription");
    }

    #[test]
    fn archive_keeps_checkpoint_and_pauses_background_until_reopened() {
        let mut host = fresh("archive-background");
        let source = host.probe_snapshot();
        host.create_world("pond", "新水域").unwrap();
        assert_eq!(host.archive_world(&host.world.world.world_id.clone(), true).err().unwrap().code, "world_active");
        let checkpoint = host.runtime.snapshot(&source.world_id).unwrap();
        host.archive_world(&source.world_id, true).unwrap();
        assert!(!host.runtime.is_world_loaded(&source.world_id));
        assert_eq!(host.switch_world(&source.world_id).err().unwrap().code, "world_archived");
        host.archive_world(&source.world_id, false).unwrap();
        let resumed = host.runtime.snapshot(&source.world_id).unwrap();
        assert_eq!(resumed.simulation_tick, checkpoint.simulation_tick);
        assert_eq!(resumed.paused, checkpoint.paused);
        assert_ne!(resumed.run_generation, checkpoint.run_generation);
    }

    #[test]
    fn autosaves_only_dirty_formal_worlds_and_recovers_paused_state() {
        const INTERVAL: u64 = 120_000 * 10_000;
        let mut host = fresh("automatic-checkpoints");
        host.pause_for_editing(true).unwrap();
        let original = host.probe_snapshot();
        host.start_trial().unwrap();
        let advanced = host.advance_trial(600).unwrap();
        host.sample_autosaves(INTERVAL);
        let mut other = WorldRuntime::new();
        let saved = other.load(&host.store, &original.world_id).unwrap();
        assert!(saved.automatic && saved.snapshot.edit_paused);
        assert_eq!(saved.snapshot.simulation_tick, original.simulation_tick);
        assert!(matches!(other.load(&host.store, &advanced.world.world_id), Err(RuntimeError::CheckpointMissing { .. })));
        host.sample_autosaves(INTERVAL);
        assert_eq!(other.load(&host.store, &original.world_id).unwrap().checkpoint_serial, saved.checkpoint_serial);
        host.discard_trial().unwrap();
        host.pause_for_editing(false).unwrap();
        host.sample_autosaves(INTERVAL);
        let resumed = other.load(&host.store, &original.world_id).unwrap();
        assert_ne!(resumed.checkpoint_serial, saved.checkpoint_serial);
        assert!(!resumed.snapshot.paused);
    }
}
