//! Opt-in checks against actual Windows/WebView windows in a debug build.

use crate::{change_all, change_visibility, exit_after_save, toggle_window, workspace, SharedHost, WINDOW_IDS};
use serde_json::{json, Value};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use world_runtime::Snapshot;

type ProbeResult<T> = Result<T, String>;

pub fn start_if_requested(app: &AppHandle) {
    if std::env::var("MICRO_ECOSYSTEM_LIFECYCLE_PROBE").as_deref() != Ok("1") {
        return;
    }
    if std::env::var_os("MICRO_ECOSYSTEM_DATA_DIR").is_none()
        || std::env::var_os("MICRO_ECOSYSTEM_TRACE_FILE").is_none()
    {
        eprintln!("Set isolated MICRO_ECOSYSTEM_DATA_DIR and MICRO_ECOSYSTEM_TRACE_FILE before running the lifecycle probe.");
        return;
    }
    let app = app.clone();
    thread::spawn(move || {
        let result = run(&app);
        report(&app, "native-probe-result", match result {
            Ok(()) => json!({ "ok": true }),
            Err(ref error) => json!({ "ok": false, "error": error }),
        });
        if result.is_ok() {
            if let Err(error) = exit_after_save(&app) {
                report(&app, "native-probe-exit-failed", json!({ "message": error.message }));
            }
        }
    });
}

fn handle_error(error: crate::host::HostError) -> String {
    format!("{}: {}", error.code, error.message)
}

fn report(app: &AppHandle, event: &str, details: Value) {
    if let Ok(mut host) = app.state::<SharedHost>().lock() {
        host.trace(event, details);
    }
}

fn snapshot(app: &AppHandle) -> ProbeResult<Snapshot> {
    Ok(app.state::<SharedHost>().lock().map_err(handle_error)?.probe_snapshot())
}

fn require(condition: bool, message: impl Into<String>) -> ProbeResult<()> {
    if condition { Ok(()) } else { Err(message.into()) }
}

fn window_states(app: &AppHandle) -> ProbeResult<Vec<Value>> {
    WINDOW_IDS.into_iter().map(|id| {
        let window = app.get_webview_window(id).ok_or_else(|| format!("missing {id} window"))?;
        let size = window.inner_size().map_err(|e| e.to_string())?;
        let decorated = window.is_decorated().map_err(|e| e.to_string())?;
        require(!decorated, format!("{id} was decorated"))?;
        #[cfg(windows)]
        let handle = format!("{:?}", window.hwnd().map_err(|e| e.to_string())?);
        #[cfg(not(windows))]
        let handle = "non-Windows".to_owned();
        Ok(json!({
            "viewId": id, "handle": handle,
            "visible": window.is_visible().map_err(|e| e.to_string())?,
            "decorated": decorated, "scaleFactor": window.scale_factor().map_err(|e| e.to_string())?,
            "physicalSize": [size.width, size.height],
        }))
    }).collect()
}

fn observe_interval(app: &AppHandle, expected_views: usize, advances: bool) -> ProbeResult<()> {
    let states = window_states(app)?;
    let visible = states.iter().filter(|view| view["visible"].as_bool() == Some(true)).count();
    require(visible == expected_views, format!("expected {expected_views} visible windows, got {visible}"))?;
    if visible == 0 {
        let count = app.state::<SharedHost>().lock().map_err(handle_error)?.probe_subscriber_count();
        require(count == 0, "hidden windows had active subscriptions")?;
    }
    let before = snapshot(app)?;
    let started = Instant::now();
    thread::sleep(Duration::from_millis(650));
    let after = snapshot(app)?;
    let advanced = after.simulated_millis > before.simulated_millis;
    let wall_millis = started.elapsed().as_millis();
    report(app, "native-probe-view-interval", json!({
        "viewCount": visible, "windowStates": states,
        "beforeTick": before.simulation_tick, "afterTick": after.simulation_tick,
        "continuousWorld": before.world_id == after.world_id && before.run_generation == after.run_generation,
        "advancedMillis": after.simulated_millis.saturating_sub(before.simulated_millis),
        "observedMillis": wall_millis,
        "expectedAdvance": advances,
    }));
    require(before.world_id == after.world_id && before.run_generation == after.run_generation, "world changed during observation")?;
    require(advanced == advances, "observed tick movement disagrees with expected pause state")
}

fn run(app: &AppHandle) -> ProbeResult<()> {
    let startup = window_states(app)?;
    report(app, "native-probe-window-metadata", json!(startup));
    require(startup.iter().all(|window| (window["visible"] == true) == (window["viewId"] == "panel")),
        "startup did not show exactly the component panel")?;
    app.state::<SharedHost>().lock().map_err(handle_error)?.pause(false).map_err(handle_error)?;

    change_all(app, false).map_err(handle_error)?;
    observe_interval(app, 0, true)?;
    change_visibility(app, "layout", true).map_err(handle_error)?;
    let initial = serde_json::to_value(workspace::panel_bounds(app).map_err(handle_error)?).map_err(|e| e.to_string())?;
    let wider = serde_json::from_value(json!({ "width": 480, "height": 260 })).map_err(|e| e.to_string())?;
    let resized = serde_json::to_value(workspace::set_panel_bounds(app, wider).map_err(handle_error)?).map_err(|e| e.to_string())?;
    let scale = resized["scaleFactor"].as_f64().ok_or("missing panel DPI scale")?;
    let resize_ok = resized["width"].as_u64() == Some((480.0 * scale).round() as u64)
        && resized["height"].as_u64() == Some((260.0 * scale).round() as u64);
    report(app, "native-probe-layout-bounds", json!({ "initial": initial, "resized": resized, "ok": resize_ok }));
    require(resize_ok, "the undecorated, non-resizable panel did not accept its layout size")?;
    let original = serde_json::from_value(json!({ "width": 384, "height": 216, "x": initial["x"], "y": initial["y"] })).map_err(|e| e.to_string())?;
    workspace::set_panel_bounds(app, original).map_err(handle_error)?;
    change_all(app, false).map_err(handle_error)?;
    change_visibility(app, "editor", true).map_err(handle_error)?;
    observe_interval(app, 1, true)?;
    toggle_window(app, "observe").map_err(handle_error)?;
    observe_interval(app, 2, true)?;
    change_all(app, true).map_err(handle_error)?;
    observe_interval(app, 3, true)?;

    app.state::<SharedHost>().lock().map_err(handle_error)?.pause(true).map_err(handle_error)?;
    change_all(app, false).map_err(handle_error)?;
    observe_interval(app, 0, false)?;

    let state = app.state::<SharedHost>();
    let mut host = state.lock().map_err(handle_error)?;
    let before = host.probe_snapshot();
    host.save().map_err(handle_error)?;
    host.restore().map_err(handle_error)?;
    let restored = host.probe_snapshot();
    let same_ecology = before.simulation_tick == restored.simulation_tick
        && before.edit_revision == restored.edit_revision
        && before.animals == restored.animals
        && before.moisture == restored.moisture
        && before.rock == restored.rock
        && before.forest == restored.forest
        && before.pond == restored.pond
        && before.season == restored.season
        && before.location == restored.location
        && before.edit_paused == restored.edit_paused
        && before.daylight_phase == restored.daylight_phase
        && before.paused == restored.paused;
    host.trace("native-probe-save-reload", json!({
        "sameEcology": same_ecology, "beforeGeneration": before.run_generation,
        "afterGeneration": restored.run_generation, "newGeneration": before.run_generation != restored.run_generation,
    }));
    require(same_ecology && before.run_generation != restored.run_generation, "restore did not preserve ecology with a new generation")
}
