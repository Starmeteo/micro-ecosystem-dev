use crate::host::{HostError, HostResult};
use crate::workspace::WorkspaceStore;
use serde::Serialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

pub const ACTION_EVENT: &str = "workspace-tray-action";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Ecology,
    Clock,
    Environment,
}

impl ComponentKind {
    fn key(self) -> &'static str {
        match self {
            Self::Ecology => "ecology",
            Self::Clock => "clock",
            Self::Environment => "environment",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CompositionMode {
    Combined,
    Free,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum TrayAction {
    Toggle { kind: ComponentKind },
    Mode { mode: CompositionMode },
    #[serde(rename = "add-ecology")]
    AddEcology,
}

impl TrayAction {
    pub fn from_menu_id(id: &str) -> Option<Self> {
        use CompositionMode::{Combined, Free, Mixed};
        use ComponentKind::{Clock, Ecology, Environment};
        Some(match id {
            "workspace-ecology" => Self::Toggle { kind: Ecology },
            "workspace-add-ecology" => Self::AddEcology,
            "workspace-clock" => Self::Toggle { kind: Clock },
            "workspace-environment" => Self::Toggle { kind: Environment },
            "workspace-combined" => Self::Mode { mode: Combined },
            "workspace-free" => Self::Mode { mode: Free },
            "workspace-mixed" => Self::Mode { mode: Mixed },
            _ => return None,
        })
    }
}

struct SavedChecks {
    layout: Option<Value>,
}

impl SavedChecks {
    fn has(&self, kind: ComponentKind) -> bool {
        self.layout.as_ref().and_then(|record| record.get("items")).and_then(Value::as_array)
            .map_or(kind == ComponentKind::Ecology, |items| {
                items.iter().any(|item| item.get("kind").and_then(Value::as_str) == Some(kind.key()))
            })
    }

    fn mode(&self) -> CompositionMode {
        match self.layout.as_ref().and_then(|record| record.get("mode")).and_then(Value::as_str) {
            Some("free") => CompositionMode::Free,
            Some("mixed") => CompositionMode::Mixed,
            _ => CompositionMode::Combined,
        }
    }
}

pub struct WorkspaceTray {
    components: [(ComponentKind, CheckMenuItem<Wry>); 3],
    modes: [(CompositionMode, CheckMenuItem<Wry>); 3],
}

impl WorkspaceTray {
    pub fn build(app: &AppHandle, layout: Option<Value>) -> tauri::Result<(Menu<Wry>, Self)> {
        use CompositionMode::{Combined, Free, Mixed};
        use ComponentKind::{Clock, Ecology, Environment};
        let checked = SavedChecks { layout };
        let component = |kind, id, label| CheckMenuItemBuilder::with_id(id, label).checked(checked.has(kind)).build(app);
        let mode = |mode, id, label| CheckMenuItemBuilder::with_id(id, label).checked(checked.mode() == mode).build(app);
        let ecology = component(Ecology, "workspace-ecology", "生态画面")?;
        let clock = component(Clock, "workspace-clock", "时钟")?;
        let environment = component(Environment, "workspace-environment", "环境信息")?;
        let combined = mode(Combined, "workspace-combined", "组合式")?;
        let free = mode(Free, "workspace-free", "自由摆放")?;
        let mixed = mode(Mixed, "workspace-mixed", "混合")?;

        let components_menu = SubmenuBuilder::new(app, "组件")
            .items(&[&ecology, &clock, &environment])
            .text("workspace-add-ecology", "添加生态组件")
            .separator()
            .items(&[&combined, &free, &mixed])
            .build()?;
        let menu = MenuBuilder::new(app)
            .item(&components_menu)
            .text("show-layout", "组件设置…")
            .separator()
            .text("show-panel", "显示组件面板")
            .text("show-observe", "打开观赏窗口")
            .text("show-editor", "打开世界编辑器")
            .separator()
            .text("hide-all", "隐藏全部窗口")
            .separator()
            .text("save-exit", "保存并退出")
            .build()?;
        Ok((menu, Self {
            components: [(Ecology, ecology), (Clock, clock), (Environment, environment)],
            modes: [(Combined, combined), (Free, free), (Mixed, mixed)],
        }))
    }

    pub fn refresh(app: &AppHandle, layout: Option<Value>) {
        let Some(tray) = app.try_state::<Self>() else { return };
        let saved = SavedChecks { layout };
        for (kind, item) in &tray.components {
            if let Err(error) = item.set_checked(saved.has(*kind)) {
                eprintln!("Could not refresh the tray component state: {error}");
            }
        }
        for (mode, item) in &tray.modes {
            if let Err(error) = item.set_checked(saved.mode() == *mode) {
                eprintln!("Could not refresh the tray composition mode: {error}");
            }
        }
    }
}

#[derive(Serialize)]
pub struct QueuedAction {
    id: String,
    #[serde(flatten)]
    action: TrayAction,
}

#[derive(Default)]
pub struct TrayActions(Mutex<PendingActions>);

#[derive(Default)]
struct PendingActions {
    last_id: u64,
    acknowledged: u64,
    actions: VecDeque<(u64, TrayAction)>,
}

impl PendingActions {
    fn checked_id(&self, id: &str) -> HostResult<u64> {
        let id = id.parse::<u64>().map_err(|_| invalid_action_id())?;
        if id == 0 || id > self.last_id {
            return Err(invalid_action_id());
        }
        Ok(id)
    }

    fn acknowledge(&mut self, id: u64) {
        if id > self.acknowledged {
            self.acknowledged = id;
            while self.actions.front().is_some_and(|(next, _)| *next <= id) {
                self.actions.pop_front();
            }
        }
    }
}

impl TrayActions {
    pub fn push(&self, action: TrayAction) -> HostResult<()> {
        let mut queue = self.lock()?;
        let next = queue.last_id.checked_add(1).ok_or_else(invalid_action_id)?;
        queue.last_id = next;
        queue.actions.push_back((next, action));
        Ok(())
    }

    pub fn read(&self) -> HostResult<Vec<QueuedAction>> {
        Ok(self.lock()?.actions.iter().map(|(id, action)| QueuedAction {
            id: id.to_string(),
            action: *action,
        }).collect())
    }

    pub fn acknowledge(&self, through_id: &str) -> HostResult<()> {
        let mut queue = self.lock()?;
        let id = queue.checked_id(through_id)?;
        queue.acknowledge(id);
        Ok(())
    }

    pub fn save_through(&self, store: &WorkspaceStore, layout: Value, through_id: &str) -> HostResult<Value> {
        let mut queue = self.lock()?;
        let id = queue.checked_id(through_id)?;
        if id <= queue.acknowledged {
            return store.load()?.ok_or_else(invalid_action_id);
        }
        let saved = store.save_panel_layout(layout)?;
        queue.acknowledge(id);
        Ok(saved)
    }

    fn lock(&self) -> HostResult<MutexGuard<'_, PendingActions>> {
        self.0.lock().map_err(|_| HostError::new("tray_actions_unavailable", "无法读取托盘组件操作"))
    }
}

fn invalid_action_id() -> HostError {
    HostError::new("invalid_tray_action_id", "托盘组件操作序号无效")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tray_checks_follow_saved_modes_and_enabled_component_kinds() {
        let default = SavedChecks { layout: None };
        assert_eq!(default.mode(), CompositionMode::Combined);
        assert!(default.has(ComponentKind::Ecology));
        assert!(!default.has(ComponentKind::Clock));
        let saved = SavedChecks { layout: Some(json!({"version":2,"mode":"mixed","items":[{"kind":"clock"}]})) };
        assert_eq!(saved.mode(), CompositionMode::Mixed);
        assert!(saved.has(ComponentKind::Clock));
        assert!(!saved.has(ComponentKind::Ecology));
        let empty = SavedChecks { layout: Some(json!({"version":2,"mode":"free","items":[]})) };
        assert_eq!(empty.mode(), CompositionMode::Free);
        assert!(!empty.has(ComponentKind::Ecology));
    }

    #[test]
    fn tray_batch_stays_queued_until_save_and_duplicate_commit_is_idempotent() {
        let root = std::env::temp_dir().join(format!("micro-ecosystem-tray-batch-{}", std::process::id()));
        let store = WorkspaceStore::new(root.clone());
        let queue = TrayActions::default();
        queue.push(TrayAction::Toggle { kind: ComponentKind::Clock }).unwrap();
        queue.push(TrayAction::Mode { mode: CompositionMode::Free }).unwrap();
        let pending = serde_json::to_value(queue.read().unwrap()).unwrap();
        assert_eq!(pending, json!([
            {"id":"1","action":"toggle","kind":"clock"},
            {"id":"2","action":"mode","mode":"free"},
        ]));
        assert!(queue.save_through(&store, json!([]), "1").is_err());
        assert_eq!(queue.read().unwrap().len(), 2);
        let visible = json!({"version":2,"mode":"combined","items":[{"kind":"clock"}]});
        assert_eq!(queue.save_through(&store, visible.clone(), "1").unwrap(), visible);
        assert_eq!(queue.read().unwrap().len(), 1);
        assert_eq!(queue.save_through(&store, json!({"items":[]}), "1").unwrap(), visible);
        assert!(queue.acknowledge("3").is_err());
        queue.acknowledge("2").unwrap();
        assert!(queue.read().unwrap().is_empty());
        std::fs::remove_file(root.join("layout.json")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
