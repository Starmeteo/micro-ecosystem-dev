#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod host;
mod power;
#[cfg(debug_assertions)]
mod probe;
mod workspace;
mod workspace_tray;

use host::{Host, HostError, HostResult, LibrarySnapshot, SharedHost, Subscription, ViewSnapshot, VIEW_IDS};
use serde_json::Value;
use std::path::PathBuf;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, EventTarget, Manager, RunEvent, State, WebviewWindow, WindowEvent};
use workspace::{PanelBounds, PanelBoundsRequest, PanelPointer, WorkspaceStore};
use workspace_tray::{QueuedAction, TrayAction, TrayActions, WorkspaceTray};
use world_runtime::solar::SolarLocation;
use world_runtime::{ForestRockCommand, PondEdit, PresetEntry, TrialRate, WorldBackupImport};

const WINDOW_IDS: [&str; 4] = ["panel", "observe", "editor", "layout"];
const LAYOUT_EVENT: &str = "workspace-layout-changed";

fn owned_label(window: &WebviewWindow, claimed: &str) -> HostResult<()> {
    if window.label() == claimed && VIEW_IDS.contains(&claimed) {
        Ok(())
    } else {
        Err(HostError::new("invalid_view", "此窗口无法控制所请求的视图绑定"))
    }
}

fn window_error(error: tauri::Error) -> HostError {
    HostError::new("window_operation_failed", error.to_string())
}

fn change_visibility(app: &AppHandle, label: &str, visible: bool) -> HostResult<()> {
    if !WINDOW_IDS.contains(&label) {
        return Err(HostError::new("invalid_view", "没有这个视图"));
    }
    let window = app.get_webview_window(label)
        .ok_or_else(|| HostError::new("view_unavailable", "当前找不到这个窗口"))?;
    if visible {
        window.unminimize().map_err(window_error)?;
        if label != "panel" {
            if let Err(error) = workspace::reveal_lost_window(&window) {
                eprintln!("{}: {}", error.code, error.message);
            }
        }
        window.show().map_err(window_error)?;
    } else {
        window.hide().map_err(window_error)?;
    }
    if let Some(state) = app.try_state::<SharedHost>() {
        state.lock()?.record_visibility(label, visible);
    }
    if visible {
        // Showing a window can succeed even when Windows declines activation.
        let _ = window.set_focus();
    }
    Ok(())
}

fn change_all(app: &AppHandle, visible: bool) -> HostResult<()> {
    let labels = if visible { VIEW_IDS.as_slice() } else { WINDOW_IDS.as_slice() };
    for &label in labels.iter().rev() {
        change_visibility(app, label, visible)?;
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn toggle_window(app: &AppHandle, label: &str) -> HostResult<()> {
    let window = app.get_webview_window(label)
        .ok_or_else(|| HostError::new("view_unavailable", "当前找不到这个窗口"))?;
    let visible = window.is_visible().map_err(window_error)?;
    let minimized = window.is_minimized().map_err(window_error)?;
    change_visibility(app, label, !visible || minimized)
}

fn exit_after_save(app: &AppHandle) -> HostResult<()> {
    app.state::<SharedHost>().lock()?.prepare_exit()?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn subscribe_world(window: WebviewWindow, state: State<'_, SharedHost>, view_id: String, world_id: Option<String>, slot: Option<String>) -> HostResult<Subscription> {
    let key = subscription_view_key(&window, &view_id, slot.as_deref())?;
    state.lock()?.subscribe_to(&key, world_id.as_deref())
}

#[tauri::command]
fn poll_world(window: WebviewWindow, state: State<'_, SharedHost>, view_id: String, subscription_token: String, after_sequence: Option<String>, slot: Option<String>) -> HostResult<Option<ViewSnapshot>> {
    let key = subscription_view_key(&window, &view_id, slot.as_deref())?;
    state.lock()?.poll(&key, &subscription_token, after_sequence.as_deref())
}

#[tauri::command]
fn unsubscribe_world(window: WebviewWindow, state: State<'_, SharedHost>, view_id: String, subscription_token: String, slot: Option<String>) -> HostResult<()> {
    let key = subscription_view_key(&window, &view_id, slot.as_deref())?;
    state.lock()?.unsubscribe(&key, &subscription_token);
    Ok(())
}

fn subscription_view_key(window: &WebviewWindow, view_id: &str, slot: Option<&str>) -> HostResult<String> {
    owned_label(window, view_id)?;
    let Some(slot) = slot else { return Ok(view_id.to_owned()); };
    if view_id != "panel" || slot.is_empty() || slot.len() > 96
        || !slot.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
        return Err(HostError::new("invalid_view", "组件视图标识无效"));
    }
    Ok(format!("panel:{slot}"))
}

#[tauri::command]
fn set_paused(state: State<'_, SharedHost>, paused: bool) -> HostResult<ViewSnapshot> {
    state.lock()?.pause(paused)
}

#[tauri::command]
fn set_edit_paused(window: WebviewWindow, state: State<'_, SharedHost>, paused: bool) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.pause_for_editing(paused)
}

#[tauri::command]
fn set_world_location(window: WebviewWindow, state: State<'_, SharedHost>, location: SolarLocation) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.set_location(location)
}

#[tauri::command]
fn create_forest_from_preset(window: WebviewWindow, state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.create_from_preset(false)
}

#[tauri::command]
fn create_pond_from_preset(window: WebviewWindow, state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.create_from_preset(true)
}

#[tauri::command]
fn list_worlds(window: WebviewWindow, state: State<'_, SharedHost>) -> HostResult<LibrarySnapshot> {
    if window.label() != "layout" { owned_label(&window, "editor")?; }
    state.lock()?.list_worlds()
}

#[tauri::command]
fn switch_world(window: WebviewWindow, state: State<'_, SharedHost>, world_id: String) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.switch_world(&world_id)
}

#[tauri::command]
fn create_world(window: WebviewWindow, state: State<'_, SharedHost>, preset_id: String, name: String) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.create_world(&preset_id, &name)
}

#[tauri::command]
fn copy_world(window: WebviewWindow, state: State<'_, SharedHost>, world_id: String, name: String) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.copy_world(&world_id, &name)
}

#[tauri::command]
fn export_world_backup(window: WebviewWindow, state: State<'_, SharedHost>, world_id: String) -> HostResult<String> {
    owned_label(&window, "editor")?;
    state.lock()?.export_world_backup(&world_id)
}

#[tauri::command]
fn import_world_backup(window: WebviewWindow, state: State<'_, SharedHost>, contents: String) -> HostResult<WorldBackupImport> {
    owned_label(&window, "editor")?;
    state.lock()?.import_world_backup(&contents)
}

#[tauri::command]
fn rename_world(window: WebviewWindow, state: State<'_, SharedHost>, world_id: String, name: String) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.rename_world(&world_id, &name)
}

#[tauri::command]
fn archive_world(window: WebviewWindow, state: State<'_, SharedHost>, world_id: String, archived: bool) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.archive_world(&world_id, archived)
}

#[tauri::command]
fn save_world_preset(window: WebviewWindow, state: State<'_, SharedHost>, name: String) -> HostResult<PresetEntry> {
    owned_label(&window, "editor")?;
    state.lock()?.save_world_preset(&name)
}

#[tauri::command]
fn observation_history(window: WebviewWindow, state: State<'_, SharedHost>, world_id: String,
    expected_generation: String) -> HostResult<world_runtime::ObservationHistory> {
    owned_label(&window, "editor")?;
    state.lock()?.observation_history(&world_id, &expected_generation)
}

#[tauri::command]
fn edit_pond(window: WebviewWindow, state: State<'_, SharedHost>, world_id: Option<String>, command_id: String, expected_generation: String,
    expected_revision: String, edit: PondEdit) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.edit_pond(world_id.as_deref(), &command_id, &expected_generation, &expected_revision, edit)
}

#[tauri::command]
fn edit_pond_history(window: WebviewWindow, state: State<'_, SharedHost>, world_id: Option<String>, command_id: String,
    expected_generation: String, expected_revision: String, redo: bool) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.edit_pond_history(world_id.as_deref(), &command_id, &expected_generation, &expected_revision, redo)
}

#[tauri::command]
fn add_starter_life(window: WebviewWindow, state: State<'_, SharedHost>, command_id: String,
    expected_generation: String, expected_revision: String) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.add_starter_life(&command_id, &expected_generation, &expected_revision)
}

#[tauri::command]
fn start_forest_trial(window: WebviewWindow, state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.start_trial()
}

#[tauri::command]
fn advance_forest_trial(window: WebviewWindow, state: State<'_, SharedHost>, ticks: u32) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.advance_trial(ticks)
}

#[tauri::command]
fn advance_trial_scaled(window: WebviewWindow, state: State<'_, SharedHost>, elapsed_ticks: u32, rate: TrialRate) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.advance_trial_scaled(elapsed_ticks, rate)
}

#[tauri::command]
fn adopt_trial(window: WebviewWindow, state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.adopt_trial()
}

#[tauri::command]
fn save_trial_as_world(window: WebviewWindow, state: State<'_, SharedHost>, name: String) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.save_trial_as_world(&name)
}

#[tauri::command]
fn discard_forest_trial(window: WebviewWindow, state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.discard_trial()
}

#[tauri::command]
fn edit_rock(state: State<'_, SharedHost>, command_id: String, expected_generation: String, expected_revision: String, x: f64, y: f64) -> HostResult<ViewSnapshot> {
    state.lock()?.edit(&command_id, &expected_generation, &expected_revision, x, y)
}

#[tauri::command]
fn edit_forest_rocks(
    window: WebviewWindow, state: State<'_, SharedHost>, world_id: String, command_id: String,
    expected_generation: String, expected_revision: String, action: ForestRockCommand,
) -> HostResult<ViewSnapshot> {
    owned_label(&window, "editor")?;
    state.lock()?.edit_forest_rocks(&world_id, &command_id, &expected_generation, &expected_revision, action)
}

#[tauri::command]
fn save_world(state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    state.lock()?.save()
}

#[tauri::command]
fn restore_world(state: State<'_, SharedHost>) -> HostResult<ViewSnapshot> {
    state.lock()?.restore()
}

#[tauri::command]
fn show_window(app: AppHandle, view_id: String) -> HostResult<bool> {
    change_visibility(&app, &view_id, true).map(|()| true)
}

#[tauri::command]
fn hide_window(app: AppHandle, view_id: String) -> HostResult<bool> {
    change_visibility(&app, &view_id, false).map(|()| true)
}

#[tauri::command]
fn show_windows(app: AppHandle) -> HostResult<bool> {
    change_all(&app, true).map(|()| true)
}

#[tauri::command]
fn hide_windows(app: AppHandle) -> HostResult<bool> {
    change_all(&app, false).map(|()| true)
}

#[tauri::command]
fn request_exit(app: AppHandle) -> HostResult<()> {
    exit_after_save(&app)
}

fn workspace_window(window: &WebviewWindow) -> HostResult<()> {
    if matches!(window.label(), "panel" | "layout") {
        Ok(())
    } else {
        Err(HostError::new("invalid_view", "此窗口不能更改组件布局或原生面板边界"))
    }
}

#[tauri::command]
fn get_workspace_layout(window: WebviewWindow, store: State<'_, WorkspaceStore>) -> HostResult<Option<Value>> {
    if window.label() != "observe" {
        workspace_window(&window)?;
    }
    store.load()
}

#[tauri::command]
fn save_workspace_layout(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, WorkspaceStore>,
    tray: State<'_, TrayActions>,
    layout: Value,
    tray_through_id: Option<String>,
) -> HostResult<Value> {
    workspace_window(&window)?;
    let saved = match (window.label(), tray_through_id) {
        ("panel", Some(id)) => tray.save_through(&store, layout, &id)?,
        ("panel", None) => store.save_panel_layout(layout)?,
        (_, Some(_)) => return Err(HostError::new("invalid_view", "仅组件面板可以确认托盘操作")),
        _ => {
            store.save(&layout)?;
            layout
        }
    };
    publish_workspace_layout(&app, &saved);
    Ok(saved)
}

#[tauri::command]
fn save_workspace_idle(
    app: AppHandle,
    window: WebviewWindow,
    store: State<'_, WorkspaceStore>,
    idle: Value,
    layout: Value,
    panel_world_id: Option<String>,
    ecology_bindings: Option<std::collections::HashMap<String, Option<String>>>,
) -> HostResult<Value> {
    if window.label() != "layout" {
        return Err(HostError::new("invalid_view", "仅组件设置窗口可以保存待机设置"));
    }
    if let Some(id) = &panel_world_id {
        if id.len() > 80 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
            return Err(HostError::new("invalid_world_id", "组件世界来源无效"));
        }
    }
    if ecology_bindings.as_ref().is_some_and(|bindings| bindings.len() > 8 || bindings.iter().any(|(piece, world)| {
        piece.is_empty() || piece.len() > 70 || world.as_ref().is_some_and(|id| id.is_empty() || id.len() > 80
            || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'))
    })) {
        return Err(HostError::new("invalid_world_id", "组件世界来源无效"));
    }
    let saved = store.save_idle(idle, layout, panel_world_id, ecology_bindings.as_ref())?;
    publish_workspace_layout(&app, &saved);
    Ok(saved)
}

fn publish_workspace_layout(app: &AppHandle, layout: &Value) {
    WorkspaceTray::refresh(app, Some(layout.clone()));
    for label in ["panel", "layout", "observe"] {
        if let Err(error) = app.emit_to(EventTarget::webview_window(label), LAYOUT_EVENT, layout) {
            eprintln!("Could not notify {label} of the saved workspace layout: {error}");
        }
    }
}

#[tauri::command]
fn get_panel_bounds(app: AppHandle, window: WebviewWindow) -> HostResult<PanelBounds> {
    workspace_window(&window)?;
    workspace::panel_bounds(&app)
}

#[tauri::command]
fn set_panel_bounds(app: AppHandle, window: WebviewWindow, bounds: PanelBoundsRequest) -> HostResult<PanelBounds> {
    workspace_window(&window)?;
    workspace::set_panel_bounds(&app, bounds)
}

#[tauri::command]
fn get_panel_pointer(window: WebviewWindow) -> HostResult<PanelPointer> {
    owned_label(&window, "panel")?;
    workspace::panel_pointer()
}

#[tauri::command]
fn get_workspace_tray_actions(window: WebviewWindow, tray: State<'_, TrayActions>) -> HostResult<Vec<QueuedAction>> {
    owned_label(&window, "panel")?;
    tray.read()
}

#[tauri::command]
fn ack_workspace_tray_actions(window: WebviewWindow, tray: State<'_, TrayActions>, through_id: String) -> HostResult<()> {
    owned_label(&window, "panel")?;
    tray.acknowledge(&through_id)
}

#[tauri::command]
fn add_workspace_ecology(app: AppHandle, window: WebviewWindow) -> HostResult<()> {
    if window.label() != "layout" {
        return Err(HostError::new("invalid_view", "仅组件设置可以请求添加生态组件"));
    }
    enqueue_workspace_action(&app, TrayAction::AddEcology)
}

fn enqueue_workspace_action(app: &AppHandle, action: TrayAction) -> HostResult<()> {
    app.state::<TrayActions>().push(action)?;
    // Windows check items flip before the callback. Keep the checks aligned
    // with the saved layout until the panel commits the queued operation.
    match app.state::<WorkspaceStore>().load() {
        Ok(layout) => WorkspaceTray::refresh(app, layout),
        Err(error) => eprintln!("{}: {}", error.code, error.message),
    }
    let panel = app.get_webview_window("panel")
        .ok_or_else(|| HostError::new("view_unavailable", "当前找不到组件面板窗口"))?;
    if !panel.is_visible().map_err(window_error)? || panel.is_minimized().map_err(window_error)? {
        change_visibility(app, "panel", true)?;
    }
    app.emit_to(EventTarget::webview_window("panel"), workspace_tray::ACTION_EVENT, action)
        .map_err(window_error)
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let layout = match app.state::<WorkspaceStore>().load() {
        Ok(layout) => layout,
        Err(error) => {
            eprintln!("{}: {}", error.code, error.message);
            None
        }
    };
    let (menu, checks) = WorkspaceTray::build(app, layout)?;
    app.manage(checks);
    let mut builder = TrayIconBuilder::with_id("micro-ecosystem")
        .tooltip("微缩生态")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let menu_id = event.id().as_ref();
            let result = if let Some(action) = TrayAction::from_menu_id(menu_id) {
                enqueue_workspace_action(app, action)
            } else {
                match menu_id {
                    "show-panel" => change_visibility(app, "panel", true),
                    "show-observe" => change_visibility(app, "observe", true),
                    "show-editor" => change_visibility(app, "editor", true),
                    "show-layout" => change_visibility(app, "layout", true),
                    "hide-all" => change_all(app, false),
                    "save-exit" => exit_after_save(app),
                    _ => return,
                }
            };
            if let Err(error) = result {
                eprintln!("{}: {}", error.code, error.message);
                let _ = change_visibility(app, "editor", true);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) {
                let _ = change_visibility(tray.app_handle(), "panel", true);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = change_visibility(app, "panel", true);
        }))
        .setup(|app| {
            let (world_data, workspace_data) = match std::env::var_os("MICRO_ECOSYSTEM_DATA_DIR") {
                Some(path) => {
                    let path = PathBuf::from(path);
                    let workspace = path.join("workspace");
                    (path, workspace)
                }
                None => {
                    let path = app.path().app_local_data_dir()?;
                    (path.join("worlds"), path.join("workspace"))
                }
            };
            let trace = std::env::var_os("MICRO_ECOSYSTEM_TRACE_FILE").map(PathBuf::from);
            let world = Host::open(world_data, trace).map_err(|error| std::io::Error::other(error.message))?;
            let shared = SharedHost::new(world);
            app.manage(shared.clone());
            app.manage(WorkspaceStore::new(workspace_data));
            app.manage(TrayActions::default());
            setup_tray(app.handle())?;
            shared.start_clock()?;
            if let Err(error) = workspace::anchor_default_panel(app.handle()) {
                eprintln!("{}: {}", error.code, error.message);
            }
            change_visibility(app.handle(), "panel", true).map_err(|error| std::io::Error::other(error.message))?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let exiting = app.try_state::<SharedHost>().is_some_and(|state| {
                    state.lock().is_ok_and(|host| host.is_exiting())
                });
                if !exiting {
                    api.prevent_close();
                    let _ = change_visibility(app, window.label(), false);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            subscribe_world, poll_world, unsubscribe_world, set_paused, edit_rock, edit_forest_rocks,
            set_edit_paused, set_world_location, create_forest_from_preset, create_pond_from_preset, edit_pond, edit_pond_history, add_starter_life,
            list_worlds, switch_world, create_world, copy_world, rename_world, archive_world, save_world_preset, observation_history,
            export_world_backup, import_world_backup,
            start_forest_trial, advance_forest_trial, advance_trial_scaled, discard_forest_trial, adopt_trial, save_trial_as_world,
            save_world, restore_world, show_window, hide_window, show_windows, hide_windows,
            request_exit, get_workspace_layout, save_workspace_layout, save_workspace_idle,
            get_panel_bounds, set_panel_bounds, get_panel_pointer,
            get_workspace_tray_actions, ack_workspace_tray_actions, add_workspace_ecology,
        ])
        .build(tauri::generate_context!())
        .expect("could not initialize micro ecosystem desktop host");

    app.run(|app, event| {
        #[cfg(debug_assertions)]
        if matches!(event, RunEvent::Ready) {
            probe::start_if_requested(app);
        }
        if let RunEvent::ExitRequested { api, .. } = event {
            if app.state::<SharedHost>().lock().is_ok_and(|host| !host.is_exiting()) {
                api.prevent_exit();
            }
        }
    });
}
