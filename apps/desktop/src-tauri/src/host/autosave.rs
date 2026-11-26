use super::*;
use world_runtime::CheckpointStamp;

const DEFAULT_INTERVAL_MS: u64 = 120_000;

pub(super) struct Autosaves {
    interval_100ns: u64,
    elapsed_100ns: u64,
    pending: VecDeque<String>,
    saved: HashMap<String, CheckpointStamp>,
    failures: HashMap<String, String>,
}

impl Autosaves {
    pub(super) fn new() -> Self {
        let interval = std::env::var("MICRO_ECOSYSTEM_AUTOSAVE_INTERVAL_MS").ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| (1_000..=3_600_000).contains(value))
            .unwrap_or(DEFAULT_INTERVAL_MS);
        Self { interval_100ns: interval * 10_000, elapsed_100ns: 0, pending: VecDeque::new(), saved: HashMap::new(), failures: HashMap::new() }
    }

    pub(super) fn remember(&mut self, runtime: &WorldRuntime, id: &str) {
        if let Ok(stamp) = runtime.checkpoint_stamp(id) { self.saved.insert(id.into(), stamp); }
        self.failures.remove(id);
    }

    pub(super) fn forget(&mut self, id: &str) {
        self.saved.remove(id);
        self.failures.remove(id);
        self.pending.retain(|pending| pending != id);
    }

    pub(super) fn failure(&self, id: &str) -> Option<String> {
        self.failures.get(id).cloned()
    }
}

impl Host {
    pub(super) fn sample_autosaves(&mut self, elapsed_100ns: u64) {
        let autosaves = &mut self.autosaves;
        autosaves.elapsed_100ns = autosaves.elapsed_100ns.saturating_add(elapsed_100ns);
        if autosaves.elapsed_100ns >= autosaves.interval_100ns && autosaves.pending.is_empty() {
            autosaves.elapsed_100ns %= autosaves.interval_100ns;
            if self.save_blocked.is_none() { autosaves.pending.push_back(self.world.world.world_id.clone()); }
            autosaves.pending.extend(self.background.iter().map(|(id, _)| id.clone()));
        }
        while let Some(id) = self.autosaves.pending.pop_front() {
            let Ok(stamp) = self.runtime.checkpoint_stamp(&id) else { continue };
            if self.autosaves.saved.get(&id) == Some(&stamp) { continue; }
            let foreground = self.world.world.world_id == id;
            match self.runtime.autosave(&self.store, &id) {
                Ok(receipt) => {
                    self.autosaves.saved.insert(id.clone(), stamp);
                    self.autosaves.failures.remove(&id);
                    if foreground { self.world.last_save_message = Some(format!("已自动保存第 {} 步", receipt.simulation_tick)); }
                    self.trace("automatic-checkpoint-saved", json!({ "affectedWorldId": id, "checkpointSerial": receipt.checkpoint_serial, "checkpointTick": receipt.simulation_tick }));
                }
                Err(error) => {
                    let message = format!("自动保存失败。{error}");
                    self.autosaves.failures.insert(id.clone(), message.clone());
                    if foreground { self.world.last_save_message = Some(message.clone()); }
                    self.trace("automatic-checkpoint-failed", json!({ "affectedWorldId": id, "message": message }));
                }
            }
            self.bump_sequence();
            // Let input and stepping run before the next dirty world's I/O.
            break;
        }
    }
}
